//! RMCP-based Integration Tests for Axon MCP Server
//!
//! This module provides integration testing using the official rmcp client SDK.

use anyhow::{Context, Result};
use clap::Parser;
use rmcp::{
    model::*,
    service::ServiceExt,
    transport::{TokioChildProcess, ConfigureCommandExt},
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    time::Duration,
};
use tokio::{process::Command, time::timeout};
use tracing::{debug, info};

/// Command line arguments for rmcp integration tests
#[derive(Parser)]
#[command(name = "axon-rmcp-tests")]
#[command(about = "RMCP-based integration tests for Axon MCP server")]
pub struct RmcpTestArgs {
    /// Path to the axon-mcp binary to test
    #[arg(short, long)]
    pub axon_binary: PathBuf,
    
    /// Directory where axon-mcp should store its SQLite database
    #[arg(short, long)]
    pub database_dir: PathBuf,
    
    /// Project root directory for axon-mcp
    #[arg(short, long)]
    pub project_root: PathBuf,
}

/// RMCP test harness using official rmcp SDK
pub struct RmcpTestHarness {
    service: rmcp::service::RunningService<rmcp::service::RoleClient, ()>,
    _database_dir: PathBuf,
    _project_root: PathBuf,
}

impl RmcpTestHarness {
    /// Create new rmcp test harness
    pub async fn new(args: RmcpTestArgs) -> Result<Self> {
        info!("🚀 Starting RMCP integration tests");
        info!("📍 Axon Binary: {:?}", args.axon_binary);
        info!("📍 Database Dir: {:?}", args.database_dir);
        info!("📍 Project Root: {:?}", args.project_root);

        // Create directories
        tokio::fs::create_dir_all(&args.database_dir).await
            .context("Failed to create database directory")?;
        tokio::fs::create_dir_all(&args.project_root).await
            .context("Failed to create project directory")?;

        // Generate database path
        let db_path = args.database_dir.join("test-axon-mcp.sqlite");
        
        info!("🔧 Setting up rmcp TokioChildProcess");

        // Set up rmcp client using TokioChildProcess
        let mut command = Command::new(&args.axon_binary);
        command.env("AXON_MCP_DB", &db_path);
        command.env("PROJECT_ROOT", &args.project_root);
        command.env("RUST_LOG", "info");
        command.current_dir(&args.project_root);

        let transport = TokioChildProcess::new(command.configure(|_| {}))?;
        
        // Create rmcp client service
        let service = ().serve(transport).await
            .context("Failed to start rmcp client service")?;

        info!("✅ RMCP client connected to server");
        info!("💾 Database: {:?}", db_path);
        info!("🔗 Server info: {:?}", service.peer_info());

        Ok(Self {
            service,
            _database_dir: args.database_dir,
            _project_root: args.project_root,
        })
    }

    /// Run all integration tests
    pub async fn run_all_tests(&mut self) -> Result<()> {
        info!("🧪 Running comprehensive rmcp integration test suite");

        // Test basic MCP functions
        self.test_health_check().await?;
        self.test_task_management().await?;
        self.test_workspace_setup().await?;
        self.test_agent_coordination().await?;
        self.test_messaging().await?;

        info!("🎉 All rmcp integration tests passed!");
        Ok(())
    }

    /// Test health check
    async fn test_health_check(&self) -> Result<()> {
        info!("🔍 Testing health_check");

        let result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "health_check".into(),
                arguments: None,
            })
        ).await
        .context("Timeout waiting for health_check response")??;

        debug!("Health check result: {:?}", result);
        
        // Parse and validate the health status
        if let Some(content) = result.content.first() {
            let text = match &content.raw {
                rmcp::model::RawContent::Text(text_content) => &text_content.text,
                _ => return Err(anyhow::anyhow!("Expected text content in health check response")),
            };
            
            let health_json: Value = serde_json::from_str(text)
                .context("Failed to parse health check response")?;
            
            if health_json["status"] != "healthy" {
                return Err(anyhow::anyhow!("Server reports unhealthy status: {:?}", health_json));
            }
        } else {
            return Err(anyhow::anyhow!("No content in health check response"));
        }

        info!("✅ health_check - PASSED");
        Ok(())
    }

    /// Test basic task management functions
    async fn test_task_management(&mut self) -> Result<()> {
        info!("🔍 Testing task management functions");

        // Test create_task
        let create_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "create_task".into(),
                arguments: Some(json!({
                    "code": "TEST-001",
                    "name": "Test Task",
                    "description": "A test task for integration testing",
                    "owner_agent_name": "test-agent"
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for create_task response")??;

        debug!("Create task result: {:?}", create_result);

        // Parse task from response
        let text = match &create_result.content[0].raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => return Err(anyhow::anyhow!("Expected text content in create task response")),
        };
        let task_json: Value = serde_json::from_str(text)
            .context("Failed to parse create task response")?;
        
        let task_id = task_json["id"].as_i64()
            .context("Task ID not found in response")?;

        info!("📝 Created task with ID: {}", task_id);

        // Test get_task_by_id
        let get_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "get_task_by_id".into(),
                arguments: Some(json!({
                    "id": task_id
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for get_task_by_id response")??;

        debug!("Get task result: {:?}", get_result);

        // Test update_task
        let update_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "update_task".into(),
                arguments: Some(json!({
                    "id": task_id,
                    "name": "Updated Test Task",
                    "description": "Updated description for testing"
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for update_task response")??;

        debug!("Update task result: {:?}", update_result);

        // Test list_tasks
        let list_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "list_tasks".into(),
                arguments: None,
            })
        ).await
        .context("Timeout waiting for list_tasks response")??;
        
        debug!("List tasks result: {:?}", list_result);

        info!("✅ Task management functions - PASSED");
        Ok(())
    }

    /// Test workspace setup functions
    async fn test_workspace_setup(&mut self) -> Result<()> {
        info!("🔍 Testing workspace setup functions");

        // Test get_setup_instructions
        let setup_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "get_setup_instructions".into(),
                arguments: Some(json!({
                    "ai_tool_type": "claude-code"
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for get_setup_instructions response")??;

        debug!("Setup instructions result: {:?}", setup_result);

        // Test get_agentic_workflow_description
        let workflow_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "get_agentic_workflow_description".into(),
                arguments: Some(json!({
                    "requested_agent_count": 3
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for get_agentic_workflow_description response")??;

        debug!("Workflow description result: {:?}", workflow_result);

        info!("✅ Workspace setup functions - PASSED");
        Ok(())
    }

    /// Test agent coordination functions
    async fn test_agent_coordination(&mut self) -> Result<()> {
        info!("🔍 Testing agent coordination functions");

        // First create a task to work with
        let create_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "create_task".into(),
                arguments: Some(json!({
                    "code": "COORD-001",
                    "name": "Coordination Test Task",
                    "description": "A task for testing agent coordination",
                    "owner_agent_name": "coord-agent"
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for create_task response")??;

        let text = match &create_result.content[0].raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => return Err(anyhow::anyhow!("Expected text content in create task response")),
        };
        let task_json: Value = serde_json::from_str(text)?;
        let task_id = task_json["id"].as_i64().unwrap();

        // Test discover_work
        let discover_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "discover_work".into(),
                arguments: Some(json!({
                    "agent_name": "test-agent",
                    "capabilities": ["rust", "testing"],
                    "max_tasks": 5
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for discover_work response")??;

        debug!("Discover work result: {:?}", discover_result);

        // Test claim_task
        let claim_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "claim_task".into(),
                arguments: Some(json!({
                    "task_id": task_id,
                    "agent_name": "test-agent"
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for claim_task response")??;

        debug!("Claim task result: {:?}", claim_result);

        // Test start_work_session
        let session_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "start_work_session".into(),
                arguments: Some(json!({
                    "task_id": task_id,
                    "agent_name": "test-agent"
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for start_work_session response")??;

        debug!("Start work session result: {:?}", session_result);

        let text = match &session_result.content[0].raw {
            rmcp::model::RawContent::Text(text_content) => &text_content.text,
            _ => return Err(anyhow::anyhow!("Expected text content in session response")),
        };
        let session_json: Value = serde_json::from_str(text)?;
        let session_id = session_json["session_id"].as_i64().unwrap();

        // Test end_work_session
        let end_session_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "end_work_session".into(),
                arguments: Some(json!({
                    "session_id": session_id,
                    "notes": "Completed testing work session",
                    "productivity_score": 0.9
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for end_work_session response")??;

        debug!("End work session result: {:?}", end_session_result);

        info!("✅ Agent coordination functions - PASSED");
        Ok(())
    }

    /// Test messaging functions
    async fn test_messaging(&mut self) -> Result<()> {
        info!("🔍 Testing messaging functions");

        // Test create_task_message
        let message_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "create_task_message".into(),
                arguments: Some(json!({
                    "task_code": "TEST-001",
                    "author_agent_name": "test-agent",
                    "target_agent_name": "other-agent",
                    "message_type": "handoff",
                    "content": "This is a test message for integration testing"
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for create_task_message response")??;

        debug!("Create message result: {:?}", message_result);

        // Test get_task_messages
        let get_messages_result = timeout(
            Duration::from_secs(10),
            self.service.peer().call_tool(CallToolRequestParam {
                name: "get_task_messages".into(),
                arguments: Some(json!({
                    "task_code": "TEST-001",
                    "limit": 10
                }).as_object().unwrap().clone()),
            })
        ).await
        .context("Timeout waiting for get_task_messages response")??;

        debug!("Get messages result: {:?}", get_messages_result);

        info!("✅ Messaging functions - PASSED");
        Ok(())
    }
}

/// Run RMCP integration tests
pub async fn run_rmcp_tests(args: RmcpTestArgs) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut harness = RmcpTestHarness::new(args).await?;
    harness.run_all_tests().await?;

    Ok(())
}