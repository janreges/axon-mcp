//! Comprehensive Integration Tests for Axon MCP Server
//!
//! This binary provides end-to-end testing of the axon-mcp server using manual Content-Length framing.

mod manual_tests;
mod rmcp_tests;

use anyhow::Result;
use manual_tests::{run_manual_tests, ManualTestArgs};
use clap::Parser;

/// Main entry point for integration tests
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = ManualTestArgs::parse();
    
    // Run manual integration tests
    run_manual_tests(args).await?;

    Ok(())
}