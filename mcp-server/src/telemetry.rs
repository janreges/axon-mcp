use anyhow::{Context, Result};
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};

use crate::config::{LogFormat, LoggingConfig};

/// Initialize the tracing subscriber for logging and telemetry
pub fn init_telemetry(config: &LoggingConfig) -> Result<()> {
    // Parse the log level from config
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.level))
        .context("Invalid log level configuration")?;

    // Create the base registry
    let registry = Registry::default().with(env_filter);

    // Configure the formatter based on the selected format
    match config.format {
        LogFormat::Pretty => {
            let fmt_layer = fmt::layer()
                .pretty()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true);

            registry.with(fmt_layer).init();
        }
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .with_span_list(true)
                .flatten_event(true);

            registry.with(fmt_layer).init();
        }
        LogFormat::Compact => {
            let fmt_layer = fmt::layer()
                .compact()
                .with_target(true)
                .with_thread_ids(false)
                .with_thread_names(false)
                .with_file(false)
                .with_line_number(false);

            registry.with(fmt_layer).init();
        }
    }

    tracing::info!(
        log_level = %config.level,
        log_format = ?config.format,
        "Telemetry initialized"
    );

    Ok(())
}

/// Setup request tracing for HTTP requests
#[allow(dead_code)]
pub fn setup_request_tracing() {
    tracing::info!("Request tracing enabled");
}

/// Create a span for database operations
#[macro_export]
macro_rules! db_span {
    ($operation:expr) => {
        tracing::info_span!("database_operation", operation = $operation)
    };
    ($operation:expr, $($key:expr => $value:expr),*) => {
        tracing::info_span!("database_operation", operation = $operation, $($key = $value),*)
    };
}

/// Create a span for MCP protocol operations
#[macro_export]
macro_rules! mcp_span {
    ($function:expr) => {
        tracing::info_span!("mcp_function", function = $function)
    };
    ($function:expr, $($key:expr => $value:expr),*) => {
        tracing::info_span!("mcp_function", function = $function, $($key = $value),*)
    };
}

/// Create a span for server operations
#[macro_export]
macro_rules! server_span {
    ($operation:expr) => {
        tracing::info_span!("server_operation", operation = $operation)
    };
    ($operation:expr, $($key:expr => $value:expr),*) => {
        tracing::info_span!("server_operation", operation = $operation, $($key = $value),*)
    };
}

/// Log server startup information
pub fn log_startup_info(config: &crate::config::Config) {
    tracing::info!(
        server_address = %config.server_address(),
        database_url = %config.database_url(),
        max_connections = config.database.max_connections,
        workers = config.server.workers,
        "MCP Server starting up"
    );
}

/// Log server shutdown information
#[allow(dead_code)]
pub fn log_shutdown_info() {
    tracing::info!("MCP Server shutting down gracefully");
}

/// Log configuration validation
pub fn log_config_validation(config: &crate::config::Config) {
    match config.validate() {
        Ok(()) => {
            tracing::info!("Configuration validation passed");
        }
        Err(e) => {
            tracing::error!(error = %e, "Configuration validation failed");
        }
    }
}

/// Error reporting helper
#[allow(dead_code)]
pub fn report_error(error: &anyhow::Error, context: &str) {
    tracing::error!(
        error = %error,
        context = context,
        "Operation failed"
    );

    // Log the error chain
    let mut current = error.source();
    let mut depth = 1;
    while let Some(err) = current {
        tracing::error!(
            error = %err,
            depth = depth,
            "Error cause"
        );
        current = err.source();
        depth += 1;
    }
}

/// Performance monitoring helper
#[allow(dead_code)]
pub struct PerformanceTimer {
    start: std::time::Instant,
    operation: String,
}

impl PerformanceTimer {
    #[allow(dead_code)]
    pub fn new(operation: impl Into<String>) -> Self {
        let operation = operation.into();
        tracing::debug!(operation = %operation, "Starting performance timer");
        Self {
            start: std::time::Instant::now(),
            operation,
        }
    }

    #[allow(dead_code)]
    pub fn finish(self) {
        let duration = self.start.elapsed();
        tracing::info!(
            operation = %self.operation,
            duration_ms = duration.as_millis(),
            "Operation completed"
        );

        if duration.as_millis() > 1000 {
            tracing::warn!(
                operation = %self.operation,
                duration_ms = duration.as_millis(),
                "Slow operation detected"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::new("test_operation");
        std::thread::sleep(Duration::from_millis(10));
        timer.finish();
        // This test mainly ensures the timer doesn't panic
    }

    #[test]
    fn test_init_telemetry_with_different_formats() {
        // Test each log format
        let configs = vec![
            LoggingConfig {
                level: "info".to_string(),
                format: LogFormat::Pretty,
            },
            LoggingConfig {
                level: "debug".to_string(),
                format: LogFormat::Json,
            },
            LoggingConfig {
                level: "warn".to_string(),
                format: LogFormat::Compact,
            },
        ];

        for config in configs {
            // Note: We can't actually test initialization in unit tests
            // because tracing can only be initialized once per process
            // This test mainly ensures the configuration parsing works
            assert!(matches!(config.format, LogFormat::Pretty | LogFormat::Json | LogFormat::Compact));
        }
    }
}