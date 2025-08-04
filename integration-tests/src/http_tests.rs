//! HTTP-based Integration Tests for Axon MCP Server
//!
//! This module provides comprehensive integration testing using the rmcp client
//! with HTTP transport to test the /mcp endpoint.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    time::Duration,
};
use tokio::{
    process::{Command, Child},
    time::{timeout, sleep},
};
use tracing::{debug, info};

/// HTTP-based test harness for Axon MCP server
pub struct HttpTestHarness {
    server_process: Option<Child>,
    server_url: String,
    project_root: PathBuf,
    server_port: u16,
}

impl HttpTestHarness {
    /// Create new HTTP test harness
    pub async fn new(
        axon_binary: PathBuf,
        project_root: PathBuf,
        server_port: u16,
    ) -> Result<Self> {
        info!("🚀 Starting HTTP MCP integration tests");
        info!("📍 Axon Binary: {:?}", axon_binary);
        info!("📍 Project Root: {:?}", project_root);
        info!("📍 Server Port: {}", server_port);

        // Create project directories
        tokio::fs::create_dir_all(&project_root).await
            .context("Failed to create project directory")?;
            
        // Generate unique project name for testing
        let uuid_string = uuid::Uuid::new_v4().to_string();
        let project_name = format!("integration-test-{}", &uuid_string[..8]);
        
        // Start the HTTP MCP server
        info!("🔧 Starting Axon MCP server for HTTP testing");
        
        let mut server_command = Command::new(&axon_binary);
        server_command
            .arg("--start")
            .arg("--port")
            .arg(server_port.to_string())
            .arg("--project")
            .arg(&project_name)
            .arg("--project-root")
            .arg(&project_root)
            .env("RUST_LOG", "info")
            .kill_on_drop(true);

        let server_process = server_command.spawn()
            .context("Failed to start Axon MCP server")?;

        let server_url = format!("http://127.0.0.1:{}/mcp", server_port);
        
        // Wait for server to be ready
        info!("⏳ Waiting for server to be ready at {}", server_url);
        let mut ready = false;
        for attempt in 1..=30 {
            sleep(Duration::from_millis(500)).await;
            
            // Try to make a simple HTTP request to check if server is running
            if let Ok(response) = reqwest::Client::new()
                .get(&format!("http://127.0.0.1:{}/health", server_port))
                .timeout(Duration::from_secs(2))
                .send()
                .await
            {
                if response.status().is_success() {
                    ready = true;
                    break;
                }
            }
            
            if attempt % 5 == 0 {
                info!("🔄 Server not ready yet, attempt {}/30", attempt);
            }
        }

        if !ready {
            return Err(anyhow::anyhow!("Server did not become ready within 15 seconds"));
        }

        info!("✅ HTTP MCP server ready at {}", server_url);

        Ok(Self {
            server_process: Some(server_process),
            server_url,
            project_root,
            server_port,
        })
    }

    /// Make an MCP JSON-RPC request to the HTTP endpoint
    async fn make_mcp_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let client = reqwest::Client::new();
        
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params.unwrap_or(json!({}))
        });

        debug!("📤 Making MCP request: {}", method);
        debug!("📄 Request body: {}", serde_json::to_string_pretty(&request_body)?);

        let response = timeout(
            Duration::from_secs(30),
            client
                .post(&self.server_url)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
        ).await
        .context("Timeout waiting for HTTP response")?
        .context("Failed to send HTTP request")?;

        let status = response.status();
        let response_text = response.text().await
            .context("Failed to read response body")?;

        debug!("📥 Response status: {}", status);
        debug!("📄 Response body: {}", response_text);

        if !status.is_success() {
            return Err(anyhow::anyhow!("HTTP request failed with status {}: {}", status, response_text));
        }

        let response_json: Value = serde_json::from_str(&response_text)
            .context("Failed to parse JSON response")?;

        // Check for JSON-RPC error
        if let Some(error) = response_json.get("error") {
            return Err(anyhow::anyhow!("MCP method {} failed: {}", method, error));
        }

        // Return the result field
        response_json.get("result")
            .ok_or_else(|| anyhow::anyhow!("No result field in JSON-RPC response"))
            .map(|r| r.clone())
    }

    /// Run all HTTP integration tests
    pub async fn run_all_tests(&mut self) -> Result<()> {
        info!("🧪 Running comprehensive HTTP MCP integration test suite");

        // Test basic MCP functions
        self.test_health_check().await?;
        self.test_task_management().await?;
        self.test_workspace_setup().await?;
        self.test_agent_coordination().await?;
        self.test_messaging().await?;
        self.test_additional_functions().await?;

        info!("🎉 All HTTP MCP integration tests passed!");
        Ok(())
    }

    /// Test health check
    async fn test_health_check(&self) -> Result<()> {
        info!("🔍 Testing health_check via HTTP");

        let result = self.make_mcp_request("health_check", None).await?;
        
        let status = result.get("status")
            .and_then(|s| s.as_str())
            .context("Missing or invalid status in health check response")?;

        if status != "healthy" {
            return Err(anyhow::anyhow!("Server reports unhealthy status: {}", status));
        }

        let version = result.get("version")
            .context("Missing version in health check response")?;
        
        info!("✅ health_check - PASSED (version: {})", version);
        Ok(())
    }

    /// Test basic task management functions
    async fn test_task_management(&mut self) -> Result<()> {
        info!("🔍 Testing task management functions via HTTP");

        // Test create_task
        let create_result = self.make_mcp_request("create_task", Some(json!({
            "code": "HTTP-TEST-001",
            "name": "HTTP Test Task",
            "description": "A test task for HTTP MCP integration testing",
            "owner_agent_name": "http-test-agent"
        }))).await?;

        let task_id = create_result.get("id")
            .and_then(|id| id.as_i64())
            .context("Task ID not found in create_task response")?;

        info!("📝 Created task with ID: {}", task_id);

        // Test get_task_by_id
        let get_result = self.make_mcp_request("get_task_by_id", Some(json!({
            "id": task_id
        }))).await?;

        let retrieved_name = get_result.get("name")
            .and_then(|n| n.as_str())
            .context("Task name not found in get_task_by_id response")?;

        if retrieved_name != "HTTP Test Task" {
            return Err(anyhow::anyhow!("Retrieved task name mismatch: {}", retrieved_name));
        }

        // Test get_task_by_code
        let get_by_code_result = self.make_mcp_request("get_task_by_code", Some(json!({
            "code": "HTTP-TEST-001"
        }))).await?;

        let code_task_id = get_by_code_result.get("id")
            .and_then(|id| id.as_i64())
            .context("Task ID not found in get_task_by_code response")?;

        if code_task_id != task_id {
            return Err(anyhow::anyhow!("Task ID mismatch when retrieving by code"));
        }

        // Test update_task
        let _update_result = self.make_mcp_request("update_task", Some(json!({
            "id": task_id,
            "name": "Updated HTTP Test Task",
            "description": "Updated description for HTTP testing"
        }))).await?;

        // Test set_task_state
        let _state_result = self.make_mcp_request("set_task_state", Some(json!({
            "id": task_id,
            "state": "InProgress"
        }))).await?;

        // Test list_tasks
        let list_result = self.make_mcp_request("list_tasks", Some(json!({
            "limit": 10
        }))).await?;
        
        // list_tasks returns array directly
        let tasks = list_result.as_array()
            .context("Expected array from list_tasks response")?;

        if tasks.is_empty() {
            return Err(anyhow::anyhow!("No tasks found in list_tasks response"));
        }

        // Test assign_task
        let _assign_result = self.make_mcp_request("assign_task", Some(json!({
            "id": task_id,
            "new_owner": "new-agent"
        }))).await?;

        info!("✅ Task management functions - PASSED");
        Ok(())
    }

    /// Test workspace setup functions
    async fn test_workspace_setup(&mut self) -> Result<()> {
        info!("🔍 Testing workspace setup functions via HTTP");

        // Test get_setup_instructions
        let setup_result = self.make_mcp_request("get_setup_instructions", Some(json!({
            "ai_tool_type": "claude-code"
        }))).await?;

        let instructions = setup_result.get("instructions")
            .context("Instructions not found in get_setup_instructions response")?;

        if instructions.as_str().unwrap_or("").is_empty() {
            return Err(anyhow::anyhow!("Empty setup instructions received"));
        }

        // Test get_agentic_workflow_description
        let workflow_result = self.make_mcp_request("get_agentic_workflow_description", Some(json!({
            "requested_agent_count": 3
        }))).await?;

        let description = workflow_result.get("description")
            .context("Description not found in get_agentic_workflow_description response")?;

        if description.as_str().unwrap_or("").is_empty() {
            return Err(anyhow::anyhow!("Empty workflow description received"));
        }

        // Test register_agent
        let _register_result = self.make_mcp_request("register_agent", Some(json!({
            "agent_name": "http-test-agent",
            "capabilities": ["rust", "testing", "http"],
            "contact_info": "test-agent@localhost"
        }))).await?;

        // Test get_instructions_for_main_ai_file
        let ai_file_result = self.make_mcp_request("get_instructions_for_main_ai_file", Some(json!({
            "ai_tool_type": "claude-code",
            "project_context": "HTTP MCP testing project"
        }))).await?;

        let ai_instructions = ai_file_result.get("instructions")
            .context("Instructions not found in get_instructions_for_main_ai_file response")?;

        if ai_instructions.as_str().unwrap_or("").is_empty() {
            return Err(anyhow::anyhow!("Empty AI file instructions received"));
        }

        info!("✅ Workspace setup functions - PASSED");
        Ok(())
    }

    /// Test agent coordination functions
    async fn test_agent_coordination(&mut self) -> Result<()> {
        info!("🔍 Testing agent coordination functions via HTTP");

        // First create a task to work with
        let create_result = self.make_mcp_request("create_task", Some(json!({
            "code": "HTTP-COORD-001",
            "name": "HTTP Coordination Test Task",
            "description": "A task for testing HTTP agent coordination",
            "required_capabilities": ["rust", "coordination"]
        }))).await?;

        let task_id = create_result.get("id")
            .and_then(|id| id.as_i64())
            .context("Task ID not found in coordination test task creation")?;

        // Test discover_work
        let discover_result = self.make_mcp_request("discover_work", Some(json!({
            "agent_name": "http-test-agent",
            "capabilities": ["rust", "testing", "coordination"],
            "max_tasks": 5
        }))).await?;

        // discover_work returns array directly
        let discovered_tasks = discover_result.as_array()
            .context("Expected array from discover_work response")?;

        info!("🔍 Discovered {} available tasks", discovered_tasks.len());

        // Test claim_task
        let claim_result = self.make_mcp_request("claim_task", Some(json!({
            "task_id": task_id,
            "agent_name": "http-test-agent"
        }))).await?;

        let claimed = claim_result.get("success")
            .and_then(|s| s.as_bool())
            .context("Success field not found in claim_task response")?;

        if !claimed {
            return Err(anyhow::anyhow!("Failed to claim task"));
        }

        // Test start_work_session
        let session_result = self.make_mcp_request("start_work_session", Some(json!({
            "task_id": task_id,
            "agent_name": "http-test-agent"
        }))).await?;

        let session_id = session_result.get("session_id")
            .and_then(|s| s.as_i64())
            .context("Session ID not found in start_work_session response")?;

        info!("🔧 Started work session with ID: {}", session_id);

        // Test end_work_session
        let _end_session_result = self.make_mcp_request("end_work_session", Some(json!({
            "session_id": session_id,
            "notes": "Completed HTTP coordination testing work session",
            "productivity_score": 0.95
        }))).await?;

        // Test release_task
        let _release_result = self.make_mcp_request("release_task", Some(json!({
            "task_id": task_id,
            "agent_name": "http-test-agent"
        }))).await?;

        info!("✅ Agent coordination functions - PASSED");
        Ok(())
    }

    /// Test messaging functions
    async fn test_messaging(&mut self) -> Result<()> {
        info!("🔍 Testing messaging functions via HTTP");

        // Test create_task_message
        let message_result = self.make_mcp_request("create_task_message", Some(json!({
            "task_code": "HTTP-TEST-001",
            "author_agent_name": "http-test-agent",
            "target_agent_name": "http-other-agent",
            "message_type": "handoff",
            "content": "This is an HTTP test message for integration testing"
        }))).await?;

        let message_id = message_result.get("id")
            .and_then(|id| id.as_i64())
            .context("Message ID not found in create_task_message response")?;

        info!("💬 Created message with ID: {}", message_id);

        // Test get_task_messages
        let get_messages_result = self.make_mcp_request("get_task_messages", Some(json!({
            "task_code": "HTTP-TEST-001",
            "limit": 10
        }))).await?;

        // get_task_messages returns array directly  
        let messages = get_messages_result.as_array()
            .context("Expected array from get_task_messages response")?;

        if messages.is_empty() {
            return Err(anyhow::anyhow!("No messages found after creating one"));
        }

        // Test targeted message retrieval
        let targeted_messages_result = self.make_mcp_request("get_task_messages", Some(json!({
            "task_code": "HTTP-TEST-001",
            "target_agent_name": "http-other-agent",
            "message_type": "handoff"
        }))).await?;

        // get_task_messages returns array directly
        let targeted_messages = targeted_messages_result.as_array()
            .context("Expected array from targeted messages")?;

        if targeted_messages.is_empty() {
            return Err(anyhow::anyhow!("No targeted messages found"));
        }

        info!("✅ Messaging functions - PASSED");
        Ok(())
    }

    /// Test additional MCP functions
    async fn test_additional_functions(&mut self) -> Result<()> {
        info!("🔍 Testing additional MCP functions via HTTP");

        // Test create_main_ai_file
        let create_ai_result = self.make_mcp_request("create_main_ai_file", Some(json!({
            "ai_tool_type": "claude-code",
            "project_context": "HTTP MCP integration test project",
            "target_filename": "CLAUDE_HTTP_TEST.md"
        }))).await?;

        let file_path = create_ai_result.get("file_path")
            .context("File path not found in create_main_ai_file response")?;

        info!("📁 Created AI file: {}", file_path);

        // Test get_workspace_manifest
        let manifest_result = self.make_mcp_request("get_workspace_manifest", None).await?;

        let manifest = manifest_result.get("manifest")
            .context("Manifest not found in get_workspace_manifest response")?;

        if manifest.as_object().unwrap_or(&serde_json::Map::new()).is_empty() {
            return Err(anyhow::anyhow!("Empty workspace manifest received"));
        }

        // Test archive_task (using a task we created earlier)
        let create_archive_task = self.make_mcp_request("create_task", Some(json!({
            "code": "HTTP-ARCHIVE-001",
            "name": "Task to Archive",
            "description": "This task will be archived for testing"
        }))).await?;

        let archive_task_id = create_archive_task.get("id")
            .and_then(|id| id.as_i64())
            .context("Archive task ID not found")?;

        let _archive_result = self.make_mcp_request("archive_task", Some(json!({
            "id": archive_task_id,
            "reason": "HTTP integration test completed"
        }))).await?;

        info!("✅ Additional MCP functions - PASSED");
        Ok(())
    }
}

impl Drop for HttpTestHarness {
    fn drop(&mut self) {
        if let Some(mut process) = self.server_process.take() {
            info!("🛑 Shutting down HTTP MCP server");
            let _ = process.kill();
        }
    }
}

/// Run comprehensive HTTP MCP integration tests
pub async fn run_http_integration_tests(
    axon_binary: PathBuf,
    project_root: PathBuf,
    server_port: u16,
) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut harness = HttpTestHarness::new(axon_binary, project_root, server_port).await?;
    harness.run_all_tests().await?;

    Ok(())
}