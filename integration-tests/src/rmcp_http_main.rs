//! RMCP HTTP MCP Integration Tests Binary
//!
//! This binary provides comprehensive RMCP-style HTTP integration testing 
//! for all 22 MCP functions via the /mcp endpoint.

mod rmcp_http_tests;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

/// Command line arguments for RMCP HTTP MCP integration tests
#[derive(Parser)]
#[command(name = "rmcp-http-mcp-tests")]
#[command(about = "RMCP-style HTTP integration tests for all 22 MCP functions")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct RmcpHttpTestArgs {
    /// Path to the axon-mcp binary to test
    #[arg(short, long, default_value = "./target/debug/axon-mcp")]
    pub axon_binary: PathBuf,
    
    /// Project root directory for axon-mcp (will be created if it doesn't exist)
    #[arg(short, long, default_value = "/tmp/axon-rmcp-http-test")]
    pub project_root: PathBuf,
    
    /// Port for the HTTP MCP server to listen on
    #[arg(long, default_value = "8892")]
    pub server_port: u16,
    
    /// Verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = RmcpHttpTestArgs::parse();
    
    // Set up logging level based on verbose flag
    let log_level = if args.verbose { "debug" } else { "info" };
    std::env::set_var("RUST_LOG", log_level);
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    info!("🚀 Starting RMCP HTTP MCP Integration Tests");
    info!("📍 Testing all 22 MCP functions via HTTP /mcp endpoint");
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
    
    // Run RMCP HTTP integration tests
    rmcp_http_tests::run_rmcp_http_integration_tests(
        args.axon_binary,
        args.project_root,
        args.server_port,
    ).await?;
    
    info!("🎉 All 22 MCP functions tested successfully via RMCP HTTP!");
    info!("📊 Test Summary:");
    info!("   ✅ Core Task Management: 9 functions");
    info!("   ✅ Advanced Coordination: 5 functions");
    info!("   ✅ Inter-Agent Messaging: 2 functions");
    info!("   ✅ Workspace Automation: 6 functions");
    info!("   📡 Transport: HTTP JSON-RPC via /mcp endpoint");
    info!("   🔗 Client: Native HTTP requests (RMCP-style)");
    
    Ok(())
}