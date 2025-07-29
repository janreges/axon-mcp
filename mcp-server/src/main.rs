mod config;
mod setup;
mod stdio;
mod telemetry;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use setup::{create_repository, initialize_app, ensure_database_directory_from_config};
use stdio::StdioMcpServer;
use telemetry::{init_telemetry, init_telemetry_with_writer, log_startup_info, log_config_validation};
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
    
    /// Transport mode: 'http' for web server (default) or 'stdio' for stdin/stdout
    #[arg(long, default_value = "http")]
    transport: String,
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
    
    // Initialize telemetry/logging system - use stderr for STDIO mode
    match cli.transport.as_str() {
        "stdio" => {
            init_telemetry_with_writer(&config.logging, std::io::stderr)
                .context("Failed to initialize telemetry")?;
        }
        _ => {
            init_telemetry(&config.logging)
                .context("Failed to initialize telemetry")?;
        }
    }
    
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

    // Handle different transport modes
    match cli.transport.as_str() {
        "stdio" => {
            // For STDIO mode, log startup to stderr only
            eprintln!("Starting MCP server in STDIO mode");
            
            // Create repository only (no HTTP server needed)
            let repository = create_repository(&config)
                .await
                .context("Failed to create repository")?;
            
            // Create STDIO server
            let stdio_server = StdioMcpServer::new(repository);
            
            // Run STDIO server (blocks until stdin is closed)
            stdio_server.serve()
                .await
                .context("STDIO MCP server error")?;
            
            eprintln!("STDIO MCP server shut down cleanly");
            Ok(())
        }
        "http" => {
            info!("Starting MCP server in HTTP mode");
            
            // Initialize application (repository and HTTP server)
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
        _ => {
            error!("Invalid transport mode: {}. Use 'http' or 'stdio'", cli.transport);
            std::process::exit(1);
        }
    }
}