//! Simple RMCP-based MCP Task Handler
//!
//! A simplified version using the official RMCP SDK following the documented patterns exactly.

use crate::serialization::*;
use ::task_core::TaskError;
use ::task_core::{
    AgentRegistration, AgenticWorkflowDescription, WorkspaceSetupService,
    HealthStatus, NewTask, TaskMessageRepository, TaskRepository, WorkspaceContextRepository,
    WorkSessionInfo,
};
use rmcp::{
    model::*,
    tool, tool_router, tool_handler,
    ServerHandler, ErrorData as McpError, ServiceExt,
};
use std::sync::Arc;

// Maximum attempts for get-or-modify loops to handle race conditions
const MAX_ATTEMPTS: u8 = 5;

/// Simple RMCP-based MCP Task Handler
#[derive(Clone)]
pub struct SimpleRmcpTaskHandler<R, M, W> {
    repository: Arc<R>,
    message_repository: Arc<M>,
    workspace_context_repository: Arc<W>,
    workspace_setup_service: WorkspaceSetupService,
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
    _project_root: Option<std::path::PathBuf>,
}

impl<R, M, W> SimpleRmcpTaskHandler<R, M, W> {
    /// Create new simple RMCP task handler
    pub fn new(
        repository: Arc<R>,
        message_repository: Arc<M>,
        workspace_context_repository: Arc<W>,
        project_root: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            repository,
            message_repository,
            workspace_context_repository: workspace_context_repository.clone(),
            workspace_setup_service: WorkspaceSetupService::new(),
            tool_router: Self::tool_router(),
            _project_root: project_root,
        }
    }

    /// Get a clone of the repository Arc for creating new handlers
    pub fn repository(&self) -> Arc<R> {
        self.repository.clone()
    }

    /// Get a clone of the message repository Arc
    pub fn message_repository(&self) -> Arc<M> {
        self.message_repository.clone()
    }
}

#[tool_router]
impl<
        R: TaskRepository + Send + Sync + 'static,
        M: TaskMessageRepository + Send + Sync + 'static,
        W: WorkspaceContextRepository + Send + Sync + 'static,
    > SimpleRmcpTaskHandler<R, M, W>
{
    /// Check server health
    #[tool(description = "Check the health status of the server")]
    async fn health_check(&self) -> std::result::Result<CallToolResult, McpError> {
        match self.repository.health_check().await {
            Ok(()) => {
                let health = HealthStatus {
                    status: "healthy".to_string(),
                    database: true,
                    protocol: true,
                    timestamp: chrono::Utc::now(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&health).unwrap(),
                )]))
            }
            Err(e) => Err(McpError::internal(format!("Health check failed: {}", e))),
        }
    }

    /// Create a new task - simplified version
    #[tool(description = "Create a new task with code, name, description, and owner")]
    async fn create_task(
        &self,
        code: String,
        name: String, 
        description: String,
        owner_agent_name: String,
    ) -> std::result::Result<CallToolResult, McpError> {
        let new_task = NewTask::new(
            code,
            name,
            description,
            Some(owner_agent_name),
        );

        match self.repository.create(new_task).await {
            Ok(task) => {
                let task_json = serialize_task_for_mcp(&task)
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_json).unwrap(),
                )]))
            }
            Err(TaskError::DuplicateKey(msg)) => Err(McpError::invalid_params(msg)),
            Err(TaskError::Validation(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Get task by ID
    #[tool(description = "Retrieve a task by its ID")]
    async fn get_task_by_id(
        &self,
        id: i32,
    ) -> std::result::Result<CallToolResult, McpError> {
        match self.repository.get_by_id(id).await {
            Ok(Some(task)) => {
                let task_json = serialize_task_for_mcp(&task)
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_json).unwrap(),
                )]))
            }
            Ok(None) => Ok(CallToolResult::success(vec![Content::text("null".to_string())])),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Get task by code
    #[tool(description = "Retrieve a task by its code")]
    async fn get_task_by_code(
        &self,
        code: String,
    ) -> std::result::Result<CallToolResult, McpError> {
        match self.repository.get_by_code(&code).await {
            Ok(Some(task)) => {
                let task_json = serialize_task_for_mcp(&task)
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_json).unwrap(),
                )]))
            }
            Ok(None) => Ok(CallToolResult::success(vec![Content::text("null".to_string())])),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// List all tasks - simplified
    #[tool(description = "List all tasks")]
    async fn list_tasks(&self) -> Result<CallToolResult, McpError> {
        use ::task_core::TaskFilter;
        
        let filter = TaskFilter {
            owner: None,
            state: None,
            created_after: None,
            created_before: None,
            completed_after: None,
            completed_before: None,
            limit: Some(100), // Default limit
        };

        match self.repository.list(filter).await {
            Ok(tasks) => {
                let task_jsons: Result<Vec<_>, _> = tasks.iter().map(serialize_task_for_mcp).collect();
                let task_jsons = task_jsons
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_jsons).unwrap(),
                )]))
            }
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Update task details
    #[tool(description = "Update task details and metadata")]
    async fn update_task(
        &self,
        id: i32,
        code: Option<String>,
        name: Option<String>,
        description: Option<String>,
        owner_agent_name: Option<String>,
        priority_score: Option<f64>,
        estimated_effort: Option<i32>,
        confidence_threshold: Option<f64>,
    ) -> std::result::Result<CallToolResult, McpError> {
        use ::task_core::UpdateTask;
        
        let updates = UpdateTask {
            code,
            name,
            description,
            owner_agent_name,
            priority_score,
            estimated_effort,
            confidence_threshold,
            ..Default::default()
        };

        match self.repository.update(id, updates).await {
            Ok(task) => {
                let task_json = serialize_task_for_mcp(&task)
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_json).unwrap(),
                )]))
            }
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(TaskError::Validation(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Set task state
    #[tool(description = "Change task lifecycle state")]
    async fn set_task_state(
        &self,
        id: i32,
        state: String,
    ) -> std::result::Result<CallToolResult, McpError> {
        use ::task_core::TaskState;
        
        let state = match state.as_str() {
            "Created" => TaskState::Created,
            "InProgress" => TaskState::InProgress,
            "Blocked" => TaskState::Blocked,
            "Review" => TaskState::Review,
            "Done" => TaskState::Done,
            "Archived" => TaskState::Archived,
            "PendingDecomposition" => TaskState::PendingDecomposition, 
            "PendingHandoff" => TaskState::PendingHandoff,
            "Quarantined" => TaskState::Quarantined,
            "WaitingForDependency" => TaskState::WaitingForDependency,
            _ => return Err(McpError::invalid_params(format!("Invalid task state: {}", state))),
        };

        match self.repository.set_state(id, state).await {
            Ok(task) => {
                let task_json = serialize_task_for_mcp(&task)
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_json).unwrap(),
                )]))
            }
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Assign task to new owner
    #[tool(description = "Transfer task ownership between agents")]
    async fn assign_task(
        &self,
        id: i32,
        new_owner: String,
    ) -> std::result::Result<CallToolResult, McpError> {
        match self.repository.assign(id, &new_owner).await {
            Ok(task) => {
                let task_json = serialize_task_for_mcp(&task)
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_json).unwrap(),
                )]))
            }
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Archive task
    #[tool(description = "Move task to archived state with audit trail")]
    async fn archive_task(
        &self,
        id: i32,
    ) -> std::result::Result<CallToolResult, McpError> {
        match self.repository.archive(id).await {
            Ok(task) => {
                let task_json = serialize_task_for_mcp(&task)
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_json).unwrap(),
                )]))
            }
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Discover available work
    #[tool(description = "Find available tasks based on agent capabilities")]
    async fn discover_work(
        &self,
        agent_name: String,
        capabilities: Vec<String>,
        max_tasks: Option<u32>,
    ) -> std::result::Result<CallToolResult, McpError> {
        let max_tasks = max_tasks.unwrap_or(10);
        
        match self.repository.discover_work(&agent_name, &capabilities, max_tasks).await {
            Ok(tasks) => {
                let task_jsons: Result<Vec<_>, _> = tasks.iter().map(serialize_task_for_mcp).collect();
                let task_jsons = task_jsons
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_jsons).unwrap(),
                )]))
            }
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Claim task atomically
    #[tool(description = "Atomically claim tasks for execution")]
    async fn claim_task(
        &self,
        task_id: i32,
        agent_name: String,
    ) -> std::result::Result<CallToolResult, McpError> {
        // Validate agent name format at protocol layer
        if agent_name.trim().is_empty() {
            return Err(McpError::invalid_params("Agent name cannot be empty"));
        }

        // Validate agent name format (kebab-case)
        if !agent_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(McpError::invalid_params(
                "Agent name must be in kebab-case format (lowercase letters, numbers, and hyphens only)"
            ));
        }

        match self.repository.claim_task(task_id, &agent_name).await {
            Ok(task) => {
                // Protocol layer validation: ensure claimed task is in InProgress state
                if task.state != ::task_core::TaskState::InProgress {
                    return Err(McpError::internal(format!(
                        "CRITICAL: claim_task succeeded but task {} is in state {:?}, expected InProgress",
                        task.id, task.state
                    )));
                }

                // Ensure task ownership is correctly set
                if task.owner_agent_name.as_deref() != Some(&agent_name) {
                    return Err(McpError::internal(format!(
                        "CRITICAL: claim_task succeeded but task {} owner is {:?}, expected Some('{}')",
                        task.id, task.owner_agent_name, agent_name
                    )));
                }

                let task_json = serialize_task_for_mcp(&task)
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_json).unwrap(),
                )]))
            }
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(TaskError::Conflict(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Release claimed task
    #[tool(description = "Release claimed tasks back to the pool")]
    async fn release_task(
        &self,
        task_id: i32,
        agent_name: String,
    ) -> std::result::Result<CallToolResult, McpError> {
        match self.repository.release_task(task_id, &agent_name).await {
            Ok(task) => {
                let task_json = serialize_task_for_mcp(&task)
                    .map_err(|e| McpError::internal(format!("Serialization error: {}", e)))?;
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&task_json).unwrap(),
                )]))
            }
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(TaskError::Conflict(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Start work session
    #[tool(description = "Begin time tracking for task work")]
    async fn start_work_session(
        &self,
        task_id: i32,
        agent_name: String,
    ) -> std::result::Result<CallToolResult, McpError> {
        match self.repository.start_work_session(task_id, &agent_name).await {
            Ok(session_id) => {
                let session_info = WorkSessionInfo {
                    session_id,
                    task_id,
                    agent_name,
                    started_at: chrono::Utc::now(),
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&session_info).unwrap(),
                )]))
            }
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(TaskError::Conflict(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// End work session
    #[tool(description = "Complete work session with productivity metrics")]
    async fn end_work_session(
        &self,
        session_id: i32,
        notes: Option<String>,
        productivity_score: Option<f64>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match self.repository.end_work_session(session_id, notes, productivity_score).await {
            Ok(()) => {
                Ok(CallToolResult::success(vec![Content::text(
                    "Work session ended successfully".to_string(),
                )]))
            }
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Create task message
    #[tool(description = "Send targeted messages between agents within tasks")]
    async fn create_task_message(
        &self,
        task_code: String,
        author_agent_name: String,
        target_agent_name: Option<String>,
        message_type: String,
        content: String,
        reply_to_message_id: Option<i32>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match self.message_repository
            .create_message(
                &task_code,
                &author_agent_name,
                target_agent_name.as_deref(),
                &message_type,
                &content,
                reply_to_message_id,
            )
            .await
        {
            Ok(message) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&message).unwrap(),
                )]))
            }
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(TaskError::Validation(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Get task messages
    #[tool(description = "Retrieve messages with advanced filtering by sender, recipient, type")]
    async fn get_task_messages(
        &self,
        task_code: String,
        author_agent_name: Option<String>,
        target_agent_name: Option<String>,
        message_type: Option<String>,
        reply_to_message_id: Option<i32>,
        limit: Option<u32>,
    ) -> std::result::Result<CallToolResult, McpError> {
        match self.message_repository
            .get_messages(
                &task_code,
                author_agent_name.as_deref(),
                target_agent_name.as_deref(),
                message_type.as_deref(),
                reply_to_message_id,
                limit,
            )
            .await
        {
            Ok(messages) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&messages).unwrap(),
                )]))
            }
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Generate AI workspace setup instructions
    #[tool(description = "Generate AI workspace setup instructions based on tool type")]
    async fn get_setup_instructions(
        &self,
        ai_tool_type: String,
    ) -> std::result::Result<CallToolResult, McpError> {
        // Parse AI tool type, default to claude-code if not provided or invalid
        let ai_tool_type = match ai_tool_type.as_str() {
            "claude-code" => ::task_core::workspace_setup::AiToolType::ClaudeCode,
            _ => ::task_core::workspace_setup::AiToolType::ClaudeCode, // Default fallback
        };

        match self
            .workspace_setup_service
            .get_setup_instructions(ai_tool_type)
            .await
        {
            Ok(response) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response.payload).unwrap(),
                )]))
            }
            Err(e) => Err(McpError::internal(format!("Workspace setup error: {}", e))),
        }
    }

    /// Get agentic workflow description  
    #[tool(description = "Analyze PRD and recommend optimal agent roles and workflow")]
    async fn get_agentic_workflow_description(
        &self,
        requested_agent_count: Option<u32>,
    ) -> std::result::Result<CallToolResult, McpError> {
        // Get agent count (default to 3 if not specified)
        let agent_count = requested_agent_count.unwrap_or(3);
        
        // Create static workflow description with agent count placeholder filled
        let workflow_prompt = format!(
            r#"# AI Agent Workflow Instructions

## Overview
You are setting up a multi-agent AI system with {} agents for MCP-based project coordination.

## Agent Coordination Strategy
1. **Agent Count**: {} agents will collaborate on this project
2. **Coordination Pattern**: Use MCP (Model Context Protocol) functions for task coordination
3. **Communication**: All agents communicate through the shared MCP task system

## Recommended Workflow Steps

### Phase 1: Discovery & Planning
- Use `list_tasks` to discover available work
- Use `discover_work` to find tasks matching your capabilities
- Use `claim_task` to claim specific tasks for your agent

### Phase 2: Execution & Coordination  
- Use `start_work_session` to begin timed work sessions
- Use `create_task_message` to communicate with other agents
- Use `get_task_messages` to read messages from team members
- Use `update_task` to report progress and status changes

### Phase 3: Handoffs & Completion
- Use `assign_task` to transfer tasks between agents
- Use `set_task_state` to mark tasks as completed or blocked
- Use `end_work_session` to close work sessions with metrics

## Agent Capabilities Template
Each of the {} agents should:
- Have distinct specializations (frontend, backend, testing, etc.)
- Monitor their assigned task queues regularly
- Communicate clearly through MCP messages
- Coordinate handoffs through task assignment

## Best Practices
- Check task status before claiming new work
- Use descriptive messages for inter-agent communication
- Set appropriate task states to keep the team informed
- Coordinate through MCP rather than direct communication

This workflow scales effectively with {} agents working in parallel."#,
            agent_count, agent_count, agent_count, agent_count
        );

        // Return simplified AgenticWorkflowDescription with static content
        let description = AgenticWorkflowDescription {
            workflow_description: workflow_prompt,
            recommended_agent_count: agent_count,
            suggested_agents: vec![], // Empty - let AI decide based on the prompt
            task_decomposition_strategy: "MCP-based coordination with parallel execution".to_string(),
            coordination_patterns: vec![
                "Task claiming and assignment".to_string(),
                "Message-based communication".to_string(),
                "Work session tracking".to_string(),
            ],
            workflow_steps: vec![
                "1. Discover available tasks using MCP functions".to_string(),
                "2. Claim tasks matching agent capabilities".to_string(),
                "3. Execute work with regular status updates".to_string(),
                "4. Coordinate handoffs through task assignment".to_string(),
                "5. Complete tasks with proper state management".to_string(),
            ],
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&description).unwrap(),
        )]))
    }

    /// Register agent in workspace
    #[tool(description = "Register an AI agent in the workspace")]
    async fn register_agent(
        &self,
        agent_name: String,
        agent_type: String,
        description: Option<String>,
        capabilities: Vec<String>,
    ) -> std::result::Result<CallToolResult, McpError> {
        use task_core::protocol::DEFAULT_WORKSPACE_ID;
        
        // Basic validation
        if agent_name.trim().is_empty() {
            return Err(McpError::invalid_params("Agent name cannot be empty"));
        }

        if let Some(ref desc) = description {
            if desc.len() > 300 {
                return Err(McpError::invalid_params("Agent description cannot exceed 300 characters"));
            }
        }

        // Validate name format (kebab-case)
        if !agent_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(McpError::invalid_params(
                "Agent name must be in kebab-case format (lowercase letters, numbers, and hyphens only)"
            ));
        }

        // Get-or-modify pattern with retry loop for race condition handling
        let mut attempt = 0u8;
        loop {
            // 1. Load existing context if present
            let maybe_context = self
                .workspace_context_repository
                .get_by_id(DEFAULT_WORKSPACE_ID)
                .await
                .map_err(|e| McpError::internal(format!("Database error: {}", e)))?;

            // 2. Check if context exists and prepare for modification
            let context_exists = maybe_context.is_some();
            let mut workspace_context = maybe_context.unwrap_or_else(|| {
                ::task_core::workspace_setup::WorkspaceContext::new(DEFAULT_WORKSPACE_ID.to_string())
            });

            // 3. Duplicate-agent guard (in case another peer registered same name first)
            if workspace_context
                .registered_agents
                .iter()
                .any(|agent| agent.name == agent_name)
            {
                return Err(McpError::invalid_params(format!(
                    "Agent with name '{}' already exists",
                    agent_name
                )));
            }

            // 4. Construct new AgentRegistration
            let agent_registration = AgentRegistration {
                name: agent_name.clone(),
                description: description.clone().unwrap_or_default(),
                prompt: format!("Agent: {}, Type: {}", agent_name, agent_type),
                capabilities: capabilities.clone(),
                ai_tool_type: ::task_core::workspace_setup::AiToolType::ClaudeCode,
                dependencies: Vec::new(),
            };

            // 5. Mutate context
            workspace_context
                .registered_agents
                .push(agent_registration.clone());
            workspace_context.updated_at = chrono::Utc::now();

            // 6. Persist with get-or-modify pattern
            let write_result = if context_exists {
                self.workspace_context_repository
                    .update(workspace_context)
                    .await
            } else {
                self.workspace_context_repository
                    .create(workspace_context)
                    .await
            };

            match write_result {
                Ok(_) => {
                    return Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&agent_registration).unwrap(),
                    )]));
                }
                Err(TaskError::DuplicateKey(_)) | Err(TaskError::Conflict(_)) => {
                    // Race condition detected
                    if attempt >= MAX_ATTEMPTS {
                        return Err(McpError::internal(format!(
                            "Workspace concurrently modified after {MAX_ATTEMPTS} attempts; please retry"
                        )));
                    }
                    attempt += 1;
                    // Small exponential back-off to reduce contention
                    tokio::time::sleep(tokio::time::Duration::from_millis(10 * attempt as u64))
                        .await;
                    continue;
                }
                Err(e) => return Err(McpError::internal(format!("Database error: {}", e))),
            }
        }
    }

    /// Get instructions for main AI file
    #[tool(description = "Get instructions for creating main AI coordination file")]
    async fn get_instructions_for_main_ai_file(
        &self,
        file_type: Option<String>,
    ) -> std::result::Result<CallToolResult, McpError> {
        // Use ClaudeCode as default AI tool type, ignore file_type for now
        match self
            .workspace_setup_service
            .get_main_file_instructions(::task_core::workspace_setup::AiToolType::ClaudeCode)
            .await
        {
            Ok(response) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response.payload).unwrap(),
                )]))
            }
            Err(e) => Err(McpError::internal(format!("Main AI file instructions error: {}", e))),
        }
    }

    /// Create main AI file
    #[tool(description = "Create the main AI coordination file (CLAUDE.md, etc.)")]
    async fn create_main_ai_file(
        &self,
        content: String,
    ) -> std::result::Result<CallToolResult, McpError> {
        use task_core::protocol::DEFAULT_WORKSPACE_ID;
        
        // Generate the main AI file (only once – outside the retry loop)
        let response = self
            .workspace_setup_service
            .create_main_file(
                &content,
                ::task_core::workspace_setup::AiToolType::ClaudeCode,
                None, // No project name provided in simplified interface
            )
            .await
            .map_err(|e| McpError::internal(format!("Main AI file creation error: {}", e)))?;

        // Pre-build the file metadata so it can be reused when we retry
        let file_metadata = ::task_core::workspace_setup::GeneratedFileMetadata {
            path: response.payload.file_name.clone(),
            description: "Main AI coordination file for Claude Code".to_string(),
            ai_tool_type: ::task_core::workspace_setup::AiToolType::ClaudeCode,
            content_type: "text/markdown".to_string(),
            created_at: chrono::Utc::now(),
        };

        // Get-or-modify pattern with retry loop for race condition handling
        let mut attempt = 0u8;
        loop {
            // 1. Fetch or create workspace context
            let maybe_context = self
                .workspace_context_repository
                .get_by_id(DEFAULT_WORKSPACE_ID)
                .await
                .map_err(|e| McpError::internal(format!("Database error: {}", e)))?;

            let context_exists = maybe_context.is_some();
            let mut workspace_context = maybe_context.unwrap_or_else(|| {
                ::task_core::workspace_setup::WorkspaceContext::new(DEFAULT_WORKSPACE_ID.to_string())
            });

            // 2. Avoid duplicate insertion on retry
            if !workspace_context
                .generated_files
                .iter()
                .any(|f| f.path == file_metadata.path)
            {
                workspace_context
                    .generated_files
                    .push(file_metadata.clone());
                workspace_context.updated_at = chrono::Utc::now();
            }

            // 3. Persist
            let write_result = if context_exists {
                self.workspace_context_repository
                    .update(workspace_context)
                    .await
            } else {
                self.workspace_context_repository
                    .create(workspace_context)
                    .await
            };

            match write_result {
                Ok(_) => {
                    return Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&response.payload).unwrap(),
                    )]));
                }
                Err(TaskError::DuplicateKey(_)) | Err(TaskError::Conflict(_)) => {
                    // Race condition detected
                    if attempt >= MAX_ATTEMPTS {
                        return Err(McpError::internal(format!(
                            "Workspace concurrently modified after {MAX_ATTEMPTS} attempts; please retry"
                        )));
                    }
                    attempt += 1;
                    // Small exponential back-off to reduce contention
                    tokio::time::sleep(tokio::time::Duration::from_millis(10 * attempt as u64))
                        .await;
                    continue; 
                }
                Err(e) => return Err(McpError::internal(format!("Database error: {}", e))),
            }
        }
    }
}

// Implement the RMCP ServerHandler trait
#[tool_handler]
impl<
        R: TaskRepository + Send + Sync + 'static,
        M: TaskMessageRepository + Send + Sync + 'static,
        W: WorkspaceContextRepository + Send + Sync + 'static,
    > ServerHandler for SimpleRmcpTaskHandler<R, M, W>
{
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simplified MCP server for AI task management. Provides basic task management functions for testing the RMCP SDK integration.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
        }
    }
}