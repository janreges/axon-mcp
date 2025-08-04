//! Minimal RMCP connection test

use anyhow::{Context, Result};
use rmcp::{
    service::ServiceExt,
    transport::{TokioChildProcess, ConfigureCommandExt},
};
use std::time::Duration;
use tokio::{process::Command, time::timeout};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("🚀 Starting minimal RMCP connection test");

    // Set up rmcp client using TokioChildProcess
    let mut command = Command::new("/Users/janreges-dev/cc-agent-testing/1/5/target/debug/axon-mcp");
    command.env("AXON_MCP_DB", "/tmp/minimal-test.sqlite");
    command.env("PROJECT_ROOT", "/tmp");
    command.env("RUST_LOG", "info");

    let transport = TokioChildProcess::new(command.configure(|_| {}))
        .context("Failed to create TokioChildProcess")?;

    info!("🔧 Connecting to server via rmcp");

    // Create rmcp client service with timeout
    let service = timeout(
        Duration::from_secs(30),
        ().serve(transport)
    ).await
    .context("Timeout waiting for rmcp client connection")?
    .context("Failed to start rmcp client service")?;

    info!("✅ RMCP client connected successfully!");
    info!("🔗 Server info: {:?}", service.peer_info());

    // Try to cancel the service gracefully
    service.cancel().await?;
    info!("✅ Connection closed gracefully");

    Ok(())
}