mod config;
mod setup;
mod telemetry;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use setup::{initialize_app, ensure_database_directory_from_config};
use telemetry::{init_telemetry, log_startup_info, log_config_validation};
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "mcp-server")]
#[command(about = "MCP Task Management Server")]
#[command(version)]
struct Cli {
    /// Configuration file path
    #[arg(short, long, env = "CONFIG_FILE")]
    config: Option<String>,
    
    /// Database URL override
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,
    
    /// Listen address override
    #[arg(long, env = "LISTEN_ADDR")]
    listen_addr: Option<String>,
    
    /// Log level override
    #[arg(long, env = "LOG_LEVEL")]
    log_level: Option<String>,
}

fn load_config(cli: &Cli) -> Result<Config> {
    let mut config = match &cli.config {
        Some(config_file) => {
            info!("Loading configuration from file: {}", config_file);
            Config::from_file(config_file)?
        }
        None => {
            info!("Loading configuration from environment");
            Config::from_env()?
        }
    };

    // Apply CLI overrides
    if let Some(ref database_url) = cli.database_url {
        info!("Overriding database URL from CLI");
        config.database.url = Some(database_url.clone());
    }

    if let Some(ref listen_addr) = cli.listen_addr {
        info!("Overriding listen address from CLI");
        config.server.listen_addr = listen_addr.clone();
    }

    if let Some(ref log_level) = cli.log_level {
        info!("Overriding log level from CLI");
        config.logging.level = log_level.clone();
    }

    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenv::dotenv().ok();
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Load configuration
    let config = load_config(&cli).context("Failed to load configuration")?;
    
    // Initialize telemetry/logging system
    init_telemetry(&config.logging).context("Failed to initialize telemetry")?;
    
    // Log configuration validation
    log_config_validation(&config);
    
    // Validate configuration (will exit if invalid)
    if let Err(e) = config.validate() {
        error!(error = %e, "Configuration validation failed");
        std::process::exit(1);
    }
    
    // Log startup information
    log_startup_info(&config);
    
    // Ensure database directory exists
    ensure_database_directory_from_config(&config)
        .context("Failed to create database directory")?;
    
    // Initialize application (repository and server)
    info!("Initializing MCP server components");
    let server = initialize_app(&config)
        .await
        .context("Failed to initialize application")?;
    
    // Create server address
    let addr = config.server_address();
    info!("Starting MCP server on {}", addr);
    
    // Setup graceful shutdown handling
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    
    // Spawn a task to handle shutdown signals
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to register SIGINT handler");
        
        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, initiating graceful shutdown");
            }
        }
        
        let _ = shutdown_tx.send(());
    });
    
    // Start the server with graceful shutdown
    tokio::select! {
        result = server.serve(&addr) => {
            match result {
                Ok(_) => {
                    info!("MCP server shut down cleanly");
                    Ok(())
                }
                Err(e) => {
                    error!(error = %e, "MCP server error");
                    std::process::exit(3);
                }
            }
        }
        _ = shutdown_rx => {
            info!("Shutdown signal received, stopping server");
            // Server will be dropped here, triggering cleanup
            Ok(())
        }
    }
}