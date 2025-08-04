//! Manual Content-Length Integration Tests for Axon MCP Server
//!
//! This module provides integration testing using manual Content-Length framing 
//! that was previously working successfully.

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::timeout,
};
use tracing::{debug, info};

/// Command line arguments for manual integration tests
#[derive(Parser)]
#[command(name = "axon-manual-tests")]
#[command(about = "Manual Content-Length integration tests for Axon MCP server")]
pub struct ManualTestArgs {
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

/// Manual test harness using Content-Length framing
pub struct ManualTestHarness {
    child: Child,
    _database_dir: PathBuf,
    _project_root: PathBuf,
}

impl ManualTestHarness {
    /// Create new manual test harness
    pub async fn new(args: ManualTestArgs) -> Result<Self> {
        info!("🚀 Starting manual Content-Length integration tests");
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
        
        info!("🔧 Starting axon-mcp server process");

        // Start axon-mcp server process
        let mut child = Command::new(&args.axon_binary)
            .env("AXON_MCP_DB", &db_path)
            .env("PROJECT_ROOT", &args.project_root)
            .env("RUST_LOG", "info")
            .current_dir(&args.project_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit()) // Show server logs
            .spawn()
            .context("Failed to start axon-mcp server")?;

        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check if the process is still running
        if let Ok(Some(exit_status)) = child.try_wait() {
            return Err(anyhow::anyhow!("Server process exited early with status: {}", exit_status));
        }

        info!("✅ Server process started successfully");
        info!("💾 Database: {:?}", db_path);

        // Perform MCP handshake
        let mut harness = Self {
            child,
            _database_dir: args.database_dir,
            _project_root: args.project_root,
        };

        harness.perform_handshake().await
            .context("Failed to complete MCP handshake")?;

        info!("✅ MCP handshake completed successfully");

        Ok(harness)
    }

    /// Perform MCP initialization handshake
    async fn perform_handshake(&mut self) -> Result<()> {
        info!("🤝 Performing MCP handshake");

        // Send initialize request
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "roots": {
                        "listChanged": true
                    }
                },
                "clientInfo": {
                    "name": "AxonIntegrationTest",
                    "version": "0.3.0"
                }
            }
        });

        self.send_message(&init_request).await
            .context("Failed to send initialize request")?;

        // Read initialize response
        let response = self.read_message().await
            .context("Failed to read initialize response")?;

        debug!("Initialize response: {}", response);

        // Parse and validate response
        let response_json: Value = serde_json::from_str(&response)
            .context("Failed to parse initialize response")?;

        if response_json["result"]["protocolVersion"] != "2024-11-05" {
            return Err(anyhow::anyhow!("Unexpected protocol version in response"));
        }

        // Send initialized notification
        let initialized_notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });

        self.send_message(&initialized_notification).await
            .context("Failed to send initialized notification")?;

        info!("✅ MCP handshake completed");
        Ok(())
    }

    /// Send MCP message with Content-Length framing
    async fn send_message(&mut self, message: &Value) -> Result<()> {
        let message_str = serde_json::to_string(message)?;
        let header = format!("Content-Length: {}\r\n\r\n", message_str.len());
        
        let stdin = self.child.stdin.as_mut()
            .context("Failed to access stdin")?;
        
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(message_str.as_bytes()).await?;
        stdin.flush().await?;
        
        debug!("Sent message: {}", message_str);
        Ok(())
    }

    /// Read MCP message with Content-Length framing
    async fn read_message(&mut self) -> Result<String> {
        let stdout = self.child.stdout.as_mut()
            .context("Failed to access stdout")?;
        
        let mut reader = BufReader::new(stdout);

        // Read headers
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            if line == "\r\n" || line == "\n" { 
                break; 
            }
            headers.push_str(&line);
        }

        // Extract Content-Length
        let content_length = headers
            .lines()
            .find_map(|l| l.strip_prefix("Content-Length:"))
            .context("Missing Content-Length header")?
            .trim()
            .parse::<usize>()
            .context("Invalid Content-Length value")?;

        // Read exact body
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).await?;
        
        let message = String::from_utf8(body)?;
        debug!("Received message: {}", message);
        Ok(message)
    }

    /// Call MCP tool with parameters
    async fn call_tool(&mut self, tool_name: &str, params: Value) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": rand::random::<u32>(),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": params
            }
        });

        self.send_message(&request).await
            .context("Failed to send tool call request")?;

        let response = timeout(
            Duration::from_secs(10),
            self.read_message()
        ).await
        .context("Timeout waiting for tool response")?
        .context("Failed to read tool response")?;

        let response_json: Value = serde_json::from_str(&response)
            .context("Failed to parse tool response")?;

        if response_json["error"].is_object() {
            return Err(anyhow::anyhow!("Tool call failed: {}", response_json["error"]));
        }

        Ok(response_json["result"].clone())
    }

    /// Run all integration tests
    pub async fn run_all_tests(&mut self) -> Result<()> {
        info!("🧪 Running comprehensive manual integration test suite");

        // Test basic MCP functions
        self.test_health_check().await?;
        self.test_task_management().await?;
        self.test_workspace_setup().await?;
        self.test_agent_coordination().await?;
        self.test_messaging().await?;

        info!("🎉 All manual integration tests passed!");
        Ok(())
    }

    /// Test health check
    async fn test_health_check(&mut self) -> Result<()> {
        info!("🔍 Testing health_check");

        let result = self.call_tool("health_check", json!({})).await?;
        debug!("Health check result: {:?}", result);
        
        // Parse and validate the health status
        if result["status"] != "healthy" {
            return Err(anyhow::anyhow!("Server reports unhealthy status: {:?}", result));
        }

        info!("✅ health_check - PASSED");
        Ok(())
    }

    /// Test basic task management functions
    async fn test_task_management(&mut self) -> Result<()> {
        info!("🔍 Testing task management functions");

        // Test create_task
        let create_result = self.call_tool("create_task", json!({
            "code": "TEST-001",
            "name": "Test Task",
            "description": "A test task for integration testing",
            "owner_agent_name": "test-agent"
        })).await?;

        debug!("Create task result: {:?}", create_result);

        // Parse task from response  
        let task_id = create_result["id"].as_i64()
            .context("Task ID not found in response")?;

        info!("📝 Created task with ID: {}", task_id);

        // Test get_task_by_id
        let get_result = self.call_tool("get_task_by_id", json!({
            "id": task_id
        })).await?;

        debug!("Get task result: {:?}", get_result);

        // Test update_task
        let update_result = self.call_tool("update_task", json!({
            "id": task_id,
            "name": "Updated Test Task",
            "description": "Updated description for testing"
        })).await?;

        debug!("Update task result: {:?}", update_result);

        // Test list_tasks
        let list_result = self.call_tool("list_tasks", json!({})).await?;
        debug!("List tasks result: {:?}", list_result);

        info!("✅ Task management functions - PASSED");
        Ok(())
    }

    /// Test workspace setup functions
    async fn test_workspace_setup(&mut self) -> Result<()> {
        info!("🔍 Testing workspace setup functions");

        // Test get_setup_instructions
        let setup_result = self.call_tool("get_setup_instructions", json!({
            "ai_tool_type": "claude-code"
        })).await?;

        debug!("Setup instructions result: {:?}", setup_result);

        // Test get_agentic_workflow_description
        let workflow_result = self.call_tool("get_agentic_workflow_description", json!({
            "requested_agent_count": 3
        })).await?;

        debug!("Workflow description result: {:?}", workflow_result);

        info!("✅ Workspace setup functions - PASSED");
        Ok(())
    }

    /// Test agent coordination functions
    async fn test_agent_coordination(&mut self) -> Result<()> {
        info!("🔍 Testing agent coordination functions");

        // First create an unassigned task to work with
        let create_result = self.call_tool("create_task", json!({
            "code": "COORD-001",
            "name": "Coordination Test Task",
            "description": "A task for testing agent coordination"
        })).await?;

        let task_id = create_result["id"].as_i64().unwrap();

        // Test discover_work with timeout mechanism
        info!("🔍 Testing discover_work - first checking for rare capabilities to trigger timeout");
        
        // First try to discover work with rare capabilities (should trigger timeout)
        let discover_future = self.call_tool("discover_work", json!({
            "agent_name": "timeout-test-agent",
            "capabilities": ["rare-capability-xyz", "nonexistent-skill"],
            "max_tasks": 5
        }));

        // Use timeout to prevent long waiting
        let discover_result = match timeout(Duration::from_secs(10), discover_future).await {
            Ok(result) => {
                info!("✅ Found existing tasks for discover_work");
                result?
            },
            Err(_) => {
                info!("⏰ Timeout waiting for discover_work (waited 10s) - creating task for agent");
                
                // Create a task that matches the agent's capabilities
                let new_task_result = self.call_tool("create_task", json!({
                    "code": "DISCOVER-TEST-001",
                    "name": "Discoverable Test Task",
                    "description": "A task created to test discover_work functionality",
                    "required_capabilities": ["rare-capability-xyz", "nonexistent-skill"]
                })).await?;
                
                info!("📝 Created discoverable task with ID: {}", new_task_result["id"]);
                
                // Now try discover_work again - should find the new task
                let retry_result = timeout(
                    Duration::from_secs(5),
                    self.call_tool("discover_work", json!({
                        "agent_name": "timeout-test-agent", 
                        "capabilities": ["rare-capability-xyz", "nonexistent-skill"],
                        "max_tasks": 5
                    }))
                ).await
                .context("Timeout even after creating task for discover_work")?
                .context("Failed to discover work after creating task")?;
                
                info!("✅ Successfully discovered work after creating task");
                retry_result
            }
        };

        debug!("Discover work result: {:?}", discover_result);

        // Test claim_task
        let claim_result = self.call_tool("claim_task", json!({
            "task_id": task_id,
            "agent_name": "test-agent"
        })).await?;

        debug!("Claim task result: {:?}", claim_result);

        // Test start_work_session
        let session_result = self.call_tool("start_work_session", json!({
            "task_id": task_id,
            "agent_name": "test-agent"
        })).await?;

        debug!("Start work session result: {:?}", session_result);

        let session_id = session_result["session_id"].as_i64().unwrap();

        // Test end_work_session
        let end_session_result = self.call_tool("end_work_session", json!({
            "session_id": session_id,
            "notes": "Completed testing work session",
            "productivity_score": 0.9
        })).await?;

        debug!("End work session result: {:?}", end_session_result);

        info!("✅ Agent coordination functions - PASSED");
        Ok(())
    }

    /// Test messaging functions
    async fn test_messaging(&mut self) -> Result<()> {
        info!("🔍 Testing messaging functions");

        // Test create_task_message
        let message_result = self.call_tool("create_task_message", json!({
            "task_code": "TEST-001",
            "author_agent_name": "test-agent",
            "target_agent_name": "other-agent",
            "message_type": "handoff",
            "content": "This is a test message for integration testing"
        })).await?;

        debug!("Create message result: {:?}", message_result);

        // Test get_task_messages
        let get_messages_result = self.call_tool("get_task_messages", json!({
            "task_code": "TEST-001",
            "limit": 10
        })).await?;

        debug!("Get messages result: {:?}", get_messages_result);

        info!("✅ Messaging functions - PASSED");
        Ok(())
    }
}

/// Terminate child process properly
impl Drop for ManualTestHarness {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// Run manual integration tests
pub async fn run_manual_tests(args: ManualTestArgs) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut harness = ManualTestHarness::new(args).await?;
    harness.run_all_tests().await?;

    Ok(())
}