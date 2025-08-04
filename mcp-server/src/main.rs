mod config;
mod self_update;
mod setup;
mod telemetry;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use setup::{
    ensure_database_directory_from_config,
    initialize_app,
};
use telemetry::{
    init_telemetry, log_config_validation, log_startup_info,
};
use tracing::{error, info};
use std::path::Path;

#[derive(Parser)]
#[command(name = "axon-mcp")]
#[command(about = "MCP Task Management Server - HTTP Only")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Start HTTP MCP server
    #[arg(long)]
    start: bool,

    /// Port to listen on
    #[arg(long, default_value = "3000")]
    port: u16,

    /// Project name for database scoping (creates axon.PROJECT_NAME.sqlite)
    #[arg(long, env = "PROJECT_NAME")]
    project: Option<String>,

    /// Project root directory (required - will create .axon/ and .claude/ subdirectories)
    #[arg(long, env = "PROJECT_ROOT")]
    project_root: Option<String>,

    /// Configuration file path
    #[arg(short, long, env = "CONFIG_FILE")]
    config: Option<String>,

    /// Database URL override (overrides --project scoping)
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Listen address override (default: 127.0.0.1)
    #[arg(long, env = "LISTEN_ADDR")]
    listen_addr: Option<String>,

    /// Log level override
    #[arg(long, env = "LOG_LEVEL")]
    log_level: Option<String>,

    /// Check for updates and install if available
    #[arg(long = "self-update")]
    self_update: bool,
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

    // Apply CLI overrides for database URL
    if let Some(ref database_url) = cli.database_url {
        info!("Overriding database URL from CLI");
        config.database.url = Some(database_url.clone());
    } else if let Some(ref project_name) = cli.project {
        // Generate project-scoped database path in .axon directory
        if let Some(ref project_root) = cli.project_root {
            let db_path = Path::new(project_root)
                .join(".axon")
                .join(format!("axon.{}.sqlite", project_name));
            let db_url = format!("sqlite://{}", db_path.display());
            info!("Using project-scoped database: {}", db_url);
            config.database.url = Some(db_url);
        }
    }

    // Apply CLI overrides for server address
    if let Some(ref listen_addr) = cli.listen_addr {
        config.server.listen_addr = listen_addr.clone();
    }
    
    // Override port from CLI
    config.server.port = cli.port;
    
    info!("Server will listen on: {}", config.server_address());

    if let Some(ref log_level) = cli.log_level {
        info!("Overriding log level from CLI");
        config.logging.level = log_level.clone();
    }

    Ok(config)
}

/// Create .axon and .claude directories in project root
fn create_project_directories(project_root: &str) -> Result<()> {
    let project_path = Path::new(project_root);
    
    // Validate project root exists
    if !project_path.exists() {
        return Err(anyhow::anyhow!("Project root directory does not exist: {}", project_root));
    }
    
    if !project_path.is_dir() {
        return Err(anyhow::anyhow!("Project root is not a directory: {}", project_root));
    }

    // Create .axon directory
    let axon_dir = project_path.join(".axon");
    if !axon_dir.exists() {
        std::fs::create_dir_all(&axon_dir)
            .with_context(|| format!("Failed to create .axon directory: {:?}", axon_dir))?;
        info!("Created .axon directory: {:?}", axon_dir);
    }

    // Create .claude directory  
    let claude_dir = project_path.join(".claude");
    if !claude_dir.exists() {
        std::fs::create_dir_all(&claude_dir)
            .with_context(|| format!("Failed to create .claude directory: {:?}", claude_dir))?;
        info!("Created .claude directory: {:?}", claude_dir);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenv::dotenv().ok();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Handle special commands first
    if cli.self_update {
        return self_update::self_update(env!("CARGO_PKG_VERSION")).await;
    }

    // Require --start flag
    if !cli.start {
        println!("🚀 Axon MCP Server - HTTP Only");
        println!();
        println!("Usage:");
        println!("  axon-mcp --start --port=8888 --project=my-project-name --project-root=/path/to/project");
        println!();
        println!("This will:");
        println!("  • Start HTTP MCP server on port 8888");
        println!("  • Use database: /path/to/project/.axon/axon.my-project-name.sqlite");
        println!("  • Create .axon/ and .claude/ directories in project root");
        println!("  • Enable structured request logging");
        println!();
        println!("For more options, use: axon-mcp --help");
        return Ok(());
    }

    // Validate required parameters
    if cli.project_root.is_none() {
        error!("--project-root parameter is required");
        std::process::exit(1);
    }

    if cli.project.is_none() && cli.database_url.is_none() {
        error!("Either --project or --database-url must be specified");
        std::process::exit(1);
    }

    // Create project directories (.axon and .claude)
    if let Some(ref project_root) = cli.project_root {
        create_project_directories(project_root)
            .context("Failed to create project directories")?;
    }

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

    // Start HTTP MCP server
    info!("🚀 Starting Axon MCP Server (HTTP Only)");
    
    if let Some(ref project_name) = cli.project {
        info!("📊 Project: {}", project_name);
        info!("💾 Database: axon.{}.sqlite", project_name);
    }
    info!("🌐 Server: http://{}", config.server_address());

    // Initialize application (repository and HTTP server)
    let server = initialize_app(&config)
        .await
        .context("Failed to initialize application")?;

    // Print ready message
    println!("✅ Axon MCP Server is ready!");
    println!("   📡 Listening on: http://{}", config.server_address());
    if let Some(ref project_name) = cli.project {
        println!("   📊 Project: {}", project_name);
        println!("   💾 Database: axon.{}.sqlite", project_name);
    }
    println!("   📋 Request logging: enabled");
    println!();
    println!("Press Ctrl+C to shutdown");
    println!();

    // Setup graceful shutdown handling
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    // Spawn a task to handle shutdown signals
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                    .expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT, initiating graceful shutdown");
                }
            }
        }

        #[cfg(windows)]
        {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl+c");
            info!("Received Ctrl+C, initiating graceful shutdown");
        }

        let _ = shutdown_tx.send(());
    });

    // Start the server with graceful shutdown
    let server_addr = config.server_address();
    tokio::select! {
        result = server.serve(&server_addr) => {
            match result {
                Ok(_) => {
                    println!("✅ Axon MCP Server shut down cleanly");
                    info!("MCP server shut down cleanly");
                    Ok(())
                }
                Err(e) => {
                    error!(error = %e, "MCP server error");
                    println!("❌ Server error: {}", e);
                    std::process::exit(3);
                }
            }
        }
        _ = shutdown_rx => {
            println!("🛑 Shutdown signal received, stopping server...");
            info!("Shutdown signal received, stopping server");
            // Server will be dropped here, triggering cleanup
            Ok(())
        }
    }
}
