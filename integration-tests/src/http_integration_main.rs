//! HTTP MCP Integration Tests Binary
//!
//! This binary provides comprehensive HTTP-based integration testing 
//! for the Axon MCP server using the /mcp endpoint.

mod http_tests;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

/// Command line arguments for HTTP MCP integration tests
#[derive(Parser)]
#[command(name = "http-mcp-tests")]
#[command(about = "HTTP-based integration tests for Axon MCP server")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct HttpTestArgs {
    /// Path to the axon-mcp binary to test
    #[arg(short, long, default_value = "./target/debug/axon-mcp")]
    pub axon_binary: PathBuf,
    
    /// Project root directory for axon-mcp (will be created if it doesn't exist)
    #[arg(short, long, default_value = "/tmp/axon-http-integration-test")]
    pub project_root: PathBuf,
    
    /// Port for the HTTP MCP server to listen on
    #[arg(long, default_value = "8891")]
    pub server_port: u16,
    
    /// Verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = HttpTestArgs::parse();
    
    // Set up logging level based on verbose flag
    let log_level = if args.verbose { "debug" } else { "info" };
    std::env::set_var("RUST_LOG", log_level);
    
    info!("🚀 Starting HTTP MCP Integration Tests");
    info!("📍 Axon Binary: {:?}", args.axon_binary);
    info!("📍 Project Root: {:?}", args.project_root);
    info!("📍 Server Port: {}", args.server_port);
    
    // Verify axon binary exists
    if !args.axon_binary.exists() {
        return Err(anyhow::anyhow!(
            "Axon binary not found at {:?}. Please build it first with: cargo build --bin axon-mcp",
            args.axon_binary
        ));
    }
    
    // Run HTTP integration tests
    http_tests::run_http_integration_tests(
        args.axon_binary,
        args.project_root,
        args.server_port,
    ).await?;
    
    info!("🎉 All HTTP MCP integration tests completed successfully!");
    Ok(())
}