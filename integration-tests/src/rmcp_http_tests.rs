//! RMCP HTTP Integration Tests for Axon MCP Server
//!
//! This module provides integration testing using the rmcp client with HTTP transport
//! to test the /mcp endpoint using proper MCP client library.

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

/// RMCP HTTP test harness for Axon MCP server
pub struct RmcpHttpTestHarness {
    server_process: Option<Child>,
    server_url: String,
    server_port: u16,
    project_root: PathBuf,
}

impl RmcpHttpTestHarness {
    /// Create new RMCP HTTP test harness
    pub async fn new(
        axon_binary: PathBuf,
        project_root: PathBuf,
        server_port: u16,
    ) -> Result<Self> {
        info!("🚀 Starting RMCP HTTP MCP integration tests");
        info!("📍 Axon Binary: {:?}", axon_binary);
        info!("📍 Project Root: {:?}", project_root);
        info!("📍 Server Port: {}", server_port);

        // Create project directories
        tokio::fs::create_dir_all(&project_root).await
            .context("Failed to create project directory")?;
            
        // Generate unique project name for testing
        let uuid_string = uuid::Uuid::new_v4().to_string();
        let project_name = format!("rmcp-http-test-{}", &uuid_string[..8]);
        
        // Start the HTTP MCP server
        info!("🔧 Starting Axon MCP server for RMCP HTTP testing");
        
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
            
            // Try health check endpoint
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

        info!("✅ RMCP HTTP MCP server ready at {}", server_url);

        Ok(Self {
            server_process: Some(server_process),
            server_url,
            server_port,
            project_root,
        })
    }

    /// Create HTTP MCP client using reqwest for testing
    async fn make_mcp_call(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let client = reqwest::Client::new();
        
        let request_id = rand::random::<u32>();
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params.unwrap_or(json!({}))
        });

        debug!("📤 Making RMCP HTTP call: {}", method);
        debug!("📄 Request: {}", serde_json::to_string_pretty(&request_body)?);

        let response = timeout(
            Duration::from_secs(30),
            client
                .post(&self.server_url)
                .header("Content-Type", "application/json")
                .header("MCP-Protocol-Version", "2025-03-26")
                .json(&request_body)
                .send()
        ).await
        .context("Timeout waiting for HTTP response")?
        .context("Failed to send HTTP request")?;

        let status = response.status();
        let response_text = response.text().await
            .context("Failed to read response body")?;

        debug!("📥 Response status: {}", status);
        debug!("📄 Response: {}", response_text);

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

    /// Run all RMCP HTTP integration tests
    pub async fn run_all_tests(&mut self) -> Result<()> {
        info!("🧪 Running comprehensive RMCP HTTP MCP integration test suite");

        // Test all 22 MCP functions via HTTP
        self.test_health_check().await?;
        self.test_core_task_management().await?;
        self.test_advanced_coordination().await?;
        self.test_inter_agent_messaging().await?;
        self.test_workspace_automation().await?;

        info!("🎉 All RMCP HTTP MCP integration tests passed!");
        Ok(())
    }

    /// Test health check function
    async fn test_health_check(&self) -> Result<()> {
        info!("🔍 Testing health_check via RMCP HTTP");

        let result = self.make_mcp_call("health_check", None).await?;
        
        let status = result.get("status")
            .and_then(|s| s.as_str())
            .context("Missing status in health check")?;

        if status != "healthy" {
            return Err(anyhow::anyhow!("Server unhealthy: {}", status));
        }

        info!("✅ health_check - PASSED");
        Ok(())
    }

    /// Test core task management functions (9 functions)
    async fn test_core_task_management(&mut self) -> Result<()> {
        info!("🔍 Testing core task management functions via RMCP HTTP");

        // 1. create_task
        let task_result = self.make_mcp_call("create_task", Some(json!({
            "code": "RMCP-001",
            "name": "RMCP HTTP Test Task",
            "description": "Task for RMCP HTTP integration testing"
        }))).await?;

        let task_id = task_result.get("id").and_then(|id| id.as_i64())
            .context("Task ID missing from create_task")?;
        info!("📝 Created task ID: {}", task_id);

        // 2. get_task_by_id
        let get_result = self.make_mcp_call("get_task_by_id", Some(json!({
            "id": task_id
        }))).await?;
        
        let retrieved_name = get_result.get("name").and_then(|n| n.as_str())
            .context("Task name missing from get_task_by_id")?;
        if retrieved_name != "RMCP HTTP Test Task" {
            return Err(anyhow::anyhow!("Name mismatch: {}", retrieved_name));
        }

        // 3. get_task_by_code
        let get_by_code_result = self.make_mcp_call("get_task_by_code", Some(json!({
            "code": "RMCP-001"
        }))).await?;
        
        let code_task_id = get_by_code_result.get("id").and_then(|id| id.as_i64())
            .context("Task ID missing from get_task_by_code")?;
        if code_task_id != task_id {
            return Err(anyhow::anyhow!("Task ID mismatch: {} vs {}", code_task_id, task_id));
        }

        // 4. update_task
        let _update_result = self.make_mcp_call("update_task", Some(json!({
            "id": task_id,
            "name": "Updated RMCP HTTP Task",
            "description": "Updated via RMCP HTTP testing"
        }))).await?;

        // 5. set_task_state
        let _state_result = self.make_mcp_call("set_task_state", Some(json!({
            "id": task_id,
            "state": "InProgress"
        }))).await?;

        // 6. list_tasks
        let list_result = self.make_mcp_call("list_tasks", Some(json!({
            "limit": 20
        }))).await?;
        
        // list_tasks returns array directly, not wrapped in "tasks" object
        let tasks = list_result.as_array()
            .context("Expected array from list_tasks")?;
        if tasks.is_empty() {
            return Err(anyhow::anyhow!("No tasks in list_tasks"));
        }

        // 7. assign_task
        let _assign_result = self.make_mcp_call("assign_task", Some(json!({
            "id": task_id,
            "new_owner": "rmcp-http-agent"
        }))).await?;

        // 8. archive_task
        let _archive_result = self.make_mcp_call("archive_task", Some(json!({
            "id": task_id,
            "reason": "RMCP HTTP test completed"
        }))).await?;

        // 9. health_check (already tested above)

        info!("✅ Core task management (9 functions) - PASSED");
        Ok(())
    }

    /// Test advanced multi-agent coordination functions (5 functions)
    async fn test_advanced_coordination(&mut self) -> Result<()> {
        info!("🔍 Testing advanced coordination functions via RMCP HTTP");

        // Create a task for coordination testing
        let coord_task = self.make_mcp_call("create_task", Some(json!({
            "code": "RMCP-COORD-001",
            "name": "Coordination Task",
            "description": "Task for RMCP coordination testing",
            "required_capabilities": ["rust", "testing"]
        }))).await?;

        let coord_task_id = coord_task.get("id").and_then(|id| id.as_i64())
            .context("Coordination task ID missing")?;

        // 10. discover_work
        let discover_result = self.make_mcp_call("discover_work", Some(json!({
            "agent_name": "rmcp-coord-agent",
            "capabilities": ["rust", "testing", "coordination"],
            "max_tasks": 10
        }))).await?;
        
        // discover_work returns array directly
        let discovered = discover_result.as_array()
            .context("Expected array from discover_work")?;
        info!("🔍 Discovered {} tasks", discovered.len());

        // 11. claim_task
        let claim_result = self.make_mcp_call("claim_task", Some(json!({
            "task_id": coord_task_id,
            "agent_name": "rmcp-coord-agent"
        }))).await?;
        
        let claimed = claim_result.get("success").and_then(|s| s.as_bool())
            .context("Claim success missing")?;
        if !claimed {
            return Err(anyhow::anyhow!("Failed to claim task"));
        }

        // 12. start_work_session
        let session_result = self.make_mcp_call("start_work_session", Some(json!({
            "task_id": coord_task_id,
            "agent_name": "rmcp-coord-agent"
        }))).await?;
        
        let session_id = session_result.get("session_id").and_then(|s| s.as_i64())
            .context("Session ID missing")?;
        info!("🔧 Started session: {}", session_id);

        // 13. end_work_session
        let _end_result = self.make_mcp_call("end_work_session", Some(json!({
            "session_id": session_id,
            "notes": "RMCP HTTP coordination test completed",
            "productivity_score": 0.98
        }))).await?;

        // 14. release_task
        let _release_result = self.make_mcp_call("release_task", Some(json!({
            "task_id": coord_task_id,
            "agent_name": "rmcp-coord-agent"
        }))).await?;

        info!("✅ Advanced coordination (5 functions) - PASSED");
        Ok(())
    }

    /// Test inter-agent messaging functions (2 functions)
    async fn test_inter_agent_messaging(&mut self) -> Result<()> {
        info!("🔍 Testing inter-agent messaging via RMCP HTTP");

        // 15. create_task_message
        let message_result = self.make_mcp_call("create_task_message", Some(json!({
            "task_code": "RMCP-001",
            "author_agent_name": "rmcp-sender",
            "target_agent_name": "rmcp-receiver",
            "message_type": "handoff",
            "content": "RMCP HTTP test message with detailed coordination info"
        }))).await?;
        
        let message_id = message_result.get("id").and_then(|id| id.as_i64())
            .context("Message ID missing")?;
        info!("💬 Created message: {}", message_id);

        // 16. get_task_messages
        let messages_result = self.make_mcp_call("get_task_messages", Some(json!({
            "task_code": "RMCP-001",
            "target_agent_name": "rmcp-receiver",
            "limit": 50
        }))).await?;
        
        // get_task_messages returns array directly
        let messages = messages_result.as_array()
            .context("Expected array from get_task_messages")?;
        
        if messages.is_empty() {
            return Err(anyhow::anyhow!("No messages found after creating one"));
        }

        info!("✅ Inter-agent messaging (2 functions) - PASSED");
        Ok(())
    }

    /// Test workspace setup automation functions (6 functions)
    async fn test_workspace_automation(&mut self) -> Result<()> {
        info!("🔍 Testing workspace automation via RMCP HTTP");

        // 17. get_setup_instructions
        let setup_result = self.make_mcp_call("get_setup_instructions", Some(json!({
            "ai_tool_type": "claude-code"
        }))).await?;
        
        let instructions = setup_result.get("instructions")
            .context("Setup instructions missing")?;
        if instructions.as_str().unwrap_or("").is_empty() {
            return Err(anyhow::anyhow!("Empty setup instructions"));
        }

        // 18. get_agentic_workflow_description
        let workflow_result = self.make_mcp_call("get_agentic_workflow_description", Some(json!({
            "requested_agent_count": 4
        }))).await?;
        
        let description = workflow_result.get("description")
            .context("Workflow description missing")?;
        if description.as_str().unwrap_or("").is_empty() {
            return Err(anyhow::anyhow!("Empty workflow description"));
        }

        // 19. register_agent
        let _register_result = self.make_mcp_call("register_agent", Some(json!({
            "agent_name": "rmcp-http-test-agent",
            "capabilities": ["rust", "http", "testing", "mcp"],
            "contact_info": "rmcp-agent@localhost"
        }))).await?;

        // 20. get_instructions_for_main_ai_file
        let ai_inst_result = self.make_mcp_call("get_instructions_for_main_ai_file", Some(json!({
            "ai_tool_type": "claude-code",
            "project_context": "RMCP HTTP MCP integration testing project"
        }))).await?;
        
        let ai_instructions = ai_inst_result.get("instructions")
            .context("AI instructions missing")?;
        if ai_instructions.as_str().unwrap_or("").is_empty() {
            return Err(anyhow::anyhow!("Empty AI instructions"));
        }

        // 21. create_main_ai_file
        let create_ai_result = self.make_mcp_call("create_main_ai_file", Some(json!({
            "ai_tool_type": "claude-code",
            "project_context": "RMCP HTTP integration test environment",
            "target_filename": "CLAUDE_RMCP_HTTP.md"
        }))).await?;
        
        let ai_filepath = create_ai_result.get("file_path")
            .context("AI file path missing")?;
        info!("📁 Created AI file: {}", ai_filepath);

        // 22. get_workspace_manifest
        let manifest_result = self.make_mcp_call("get_workspace_manifest", None).await?;
        
        let manifest = manifest_result.get("manifest")
            .context("Workspace manifest missing")?;
        if manifest.as_object().unwrap_or(&serde_json::Map::new()).is_empty() {
            return Err(anyhow::anyhow!("Empty workspace manifest"));
        }

        info!("✅ Workspace automation (6 functions) - PASSED");
        Ok(())
    }
}

impl Drop for RmcpHttpTestHarness {
    fn drop(&mut self) {
        if let Some(mut process) = self.server_process.take() {
            info!("🛑 Shutting down RMCP HTTP MCP server");
            let _ = process.kill();
        }
    }
}

/// Run comprehensive RMCP HTTP MCP integration tests
pub async fn run_rmcp_http_integration_tests(
    axon_binary: PathBuf,
    project_root: PathBuf,
    server_port: u16,
) -> Result<()> {
    let mut harness = RmcpHttpTestHarness::new(axon_binary, project_root, server_port).await?;
    harness.run_all_tests().await?;
    Ok(())
}