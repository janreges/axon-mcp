//! RMCP-based MCP Task Handler
//!
//! Uses the official RMCP SDK to implement MCP protocol handling with #[tool] macros.

use crate::serialization::*;
use ::task_core::error::Result;
use ::task_core::TaskError;
use ::task_core::{
    AgentRegistration, AgenticWorkflowDescription, WorkspaceSetupService,
};
use ::task_core::{
    WorkSessionInfo,
};
use ::task_core::{
    HealthStatus, NewTask, TaskMessageRepository,
    TaskRepository, WorkspaceContextRepository,
};
use rmcp::{
    model::*,
    tool, tool_router, tool_handler,
    ServerHandler, ErrorData as McpError,
};
use std::future::Future;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Maximum attempts for get-or-modify loops to handle race conditions
const MAX_ATTEMPTS: u8 = 5;

/// RMCP-based MCP Task Handler
#[derive(Clone)]
pub struct RmcpTaskHandler<R, M, W> {
    repository: Arc<R>,
    message_repository: Arc<M>,
    workspace_context_repository: Arc<W>,
    workspace_setup_service: WorkspaceSetupService,
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
    _project_root: Option<std::path::PathBuf>,
}

/// Create Task Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateTaskRequest {
    #[schemars(description = "Human-readable task identifier")]
    pub code: String,
    #[schemars(description = "Brief task title")]
    pub name: String,
    #[schemars(description = "Detailed task requirements")]
    pub description: String,
    #[schemars(description = "Agent identifier who owns this task")]
    pub owner_agent_name: String,
}

/// Update Task Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTaskRequest {
    #[schemars(description = "Task ID to update")]
    pub id: i32,
    #[schemars(description = "New task title")]
    pub name: Option<String>,
    #[schemars(description = "New task description")]
    pub description: Option<String>,
    #[schemars(description = "New owner agent name")]
    pub owner_agent_name: Option<String>,
}

/// Set Task State Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetTaskStateRequest {
    #[schemars(description = "Task ID to update")]
    pub id: i32,
    #[schemars(description = "New task state")]
    pub state: String,
}

/// Get Task by ID Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTaskByIdRequest {
    #[schemars(description = "Task ID to retrieve")]
    pub id: i32,
}

/// Get Task by Code Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTaskByCodeRequest {
    #[schemars(description = "Task code to retrieve")]
    pub code: String,
}

/// List Tasks Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListTasksRequest {
    #[schemars(description = "Filter by owner agent name")]
    pub owner: Option<String>,
    #[schemars(description = "Filter by task state")]
    pub state: Option<String>,
    #[schemars(description = "Filter tasks created after this date")]
    pub created_after: Option<String>,
    #[schemars(description = "Filter tasks created before this date")]
    pub created_before: Option<String>,
    #[schemars(description = "Filter tasks completed after this date")]
    pub completed_after: Option<String>,
    #[schemars(description = "Filter tasks completed before this date")]
    pub completed_before: Option<String>,
    #[schemars(description = "Maximum number of tasks to return")]
    pub limit: Option<u32>,
}

/// Assign Task Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssignTaskRequest {
    #[schemars(description = "Task ID to assign")]
    pub id: i32,
    #[schemars(description = "New owner agent name")]
    pub new_owner: String,
}

/// Archive Task Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArchiveTaskRequest {
    #[schemars(description = "Task ID to archive")]
    pub id: i32,
}

/// Discover Work Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiscoverWorkRequest {
    #[schemars(description = "Agent name requesting work")]
    pub agent_name: String,
    #[schemars(description = "Agent capabilities for task matching")]
    pub capabilities: Vec<String>,
    #[schemars(description = "Maximum number of tasks to return")]
    pub max_tasks: Option<u32>,
}

/// Claim Task Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClaimTaskRequest {
    #[schemars(description = "Task ID to claim")]
    pub task_id: i32,
    #[schemars(description = "Agent name claiming the task")]
    pub agent_name: String,
}

/// Release Task Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseTaskRequest {
    #[schemars(description = "Task ID to release")]
    pub task_id: i32,
    #[schemars(description = "Agent name releasing the task")]
    pub agent_name: String,
}

/// Start Work Session Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StartWorkSessionRequest {
    #[schemars(description = "Task ID for the work session")]
    pub task_id: i32,
    #[schemars(description = "Agent name starting the session")]
    pub agent_name: String,
}

/// End Work Session Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EndWorkSessionRequest {
    #[schemars(description = "Work session ID to end")]
    pub session_id: i32,
    #[schemars(description = "Optional notes about the work session")]
    pub notes: Option<String>,
    #[schemars(description = "Optional productivity score (0.0 to 1.0)")]
    pub productivity_score: Option<f64>,
}

/// Create Task Message Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateTaskMessageRequest {
    #[schemars(description = "Task code for the message")]
    pub task_code: String,
    #[schemars(description = "Agent name sending the message")]
    pub author_agent_name: String,
    #[schemars(description = "Target agent name (optional for broadcasts)")]
    pub target_agent_name: Option<String>,
    #[schemars(description = "Message type (handoff, comment, question, etc.)")]
    pub message_type: String,
    #[schemars(description = "Message content")]
    pub content: String,
    #[schemars(description = "Optional message ID this is replying to")]
    pub reply_to_message_id: Option<i32>,
}

/// Get Task Messages Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetTaskMessagesRequest {
    #[schemars(description = "Task code to get messages for")]
    pub task_code: String,
    #[schemars(description = "Filter by author agent name")]
    pub author_agent_name: Option<String>,
    #[schemars(description = "Filter by target agent name")]
    pub target_agent_name: Option<String>,
    #[schemars(description = "Filter by message type")]
    pub message_type: Option<String>,
    #[schemars(description = "Filter by reply to message ID")]
    pub reply_to_message_id: Option<i32>,
    #[schemars(description = "Maximum number of messages to return")]
    pub limit: Option<u32>,
}

/// Get Setup Instructions Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetSetupInstructionsRequest {
    #[schemars(description = "AI tool type (e.g., 'claude-code')")]
    pub ai_tool_type: String,
}

/// Get Agentic Workflow Description Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetAgenticWorkflowDescriptionRequest {
    #[schemars(description = "Requested number of agents for the workflow")]
    pub requested_agent_count: Option<u32>,
}

/// Register Agent Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegisterAgentRequest {
    #[schemars(description = "Agent name (kebab-case format)")]
    pub agent_name: String,
    #[schemars(description = "Agent type (coordinator, developer, tester, etc.)")]
    pub agent_type: String,
    #[schemars(description = "Agent capabilities")]
    pub capabilities: Vec<String>,
    #[schemars(description = "Optional agent description")]
    pub description: Option<String>,
}

/// Get Instructions for Main AI File Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetInstructionsForMainAiFileRequest {
    #[schemars(description = "File type (e.g., 'claude-md')")]
    pub file_type: Option<String>,
}

/// Create Main AI File Parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateMainAiFileRequest {
    #[schemars(description = "Content for the main AI file")]
    pub content: String,
}

impl<R, M, W> RmcpTaskHandler<R, M, W> {
    /// Create new RMCP task handler
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
    > RmcpTaskHandler<R, M, W>
{
    /// Create a new task
    #[tool(description = "Create a new task with code, name, description, and owner")]
    async fn create_task(
        &self,
        request: CreateTaskRequest,
    ) -> Result<CallToolResult, McpError> {
        let new_task = NewTask::new(
            request.code,
            request.name,
            request.description,
            Some(request.owner_agent_name),
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

    /// Update an existing task
    #[tool(description = "Update an existing task's properties")]
    async fn update_task(
        &self,
        request: UpdateTaskRequest,
    ) -> Result<CallToolResult, McpError> {
        let update_data = UpdateTaskParams {
            id: request.id,
            name: request.name,
            description: request.description,
            owner_agent_name: request.owner_agent_name,
        }.into_update_data();

        match self.repository.update(request.id, update_data).await {
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
    #[tool(description = "Set the state of a task")]
    async fn set_task_state(
        &self,
        request: SetTaskStateRequest,
    ) -> Result<CallToolResult, McpError> {
        use ::task_core::TaskState;
        
        let state = match request.state.as_str() {
            "Created" => TaskState::Created,
            "InProgress" => TaskState::InProgress,
            "Blocked" => TaskState::Blocked,
            "Review" => TaskState::Review,
            "Done" => TaskState::Done,
            "Archived" => TaskState::Archived,
            _ => return Err(McpError::invalid_params(format!("Invalid state: {}", request.state))),
        };

        match self.repository.set_state(request.id, state).await {
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

    /// Get task by ID
    #[tool(description = "Retrieve a task by its ID")]
    async fn get_task_by_id(
        &self,
        request: GetTaskByIdRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.repository.get_by_id(request.id).await {
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
        request: GetTaskByCodeRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.repository.get_by_code(&request.code).await {
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

    /// List tasks with optional filtering
    #[tool(description = "List tasks with optional filtering by owner, state, date range, and limit")]
    async fn list_tasks(
        &self,
        request: ListTasksRequest,
    ) -> Result<CallToolResult, McpError> {
        let params = ListTasksParams {
            owner: request.owner,
            state: request.state,
            created_after: request.created_after,
            created_before: request.created_before,
            completed_after: request.completed_after,
            completed_before: request.completed_before,
            limit: request.limit,
        };

        let filter = params.to_task_filter()
            .map_err(|e| McpError::invalid_params(format!("Filter error: {}", e)))?;

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

    /// Assign task to a different agent
    #[tool(description = "Assign a task to a different agent")]
    async fn assign_task(
        &self,
        request: AssignTaskRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.repository.assign(request.id, &request.new_owner).await {
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

    /// Archive a completed task
    #[tool(description = "Archive a completed task")]
    async fn archive_task(
        &self,
        request: ArchiveTaskRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.repository.archive(request.id).await {
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

    /// Check server health
    #[tool(description = "Check the health status of the server")]
    async fn health_check(&self) -> Result<CallToolResult, McpError> {
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

    /// Discover available work based on agent capabilities
    #[tool(description = "Discover available tasks based on agent capabilities")]
    async fn discover_work(
        &self,
        request: DiscoverWorkRequest,
    ) -> Result<CallToolResult, McpError> {
        let max_tasks = request.max_tasks.unwrap_or(10);
        
        match self.repository.discover_work(&request.agent_name, &request.capabilities, max_tasks).await {
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

    /// Atomically claim a task for execution
    #[tool(description = "Atomically claim a task for execution")]
    async fn claim_task(
        &self,
        request: ClaimTaskRequest,
    ) -> Result<CallToolResult, McpError> {
        // Validate agent name format at protocol layer
        if request.agent_name.trim().is_empty() {
            return Err(McpError::invalid_params("Agent name cannot be empty".to_string()));
        }

        // Validate agent name format (kebab-case)
        if !request
            .agent_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(McpError::invalid_params(
                "Agent name must be in kebab-case format (lowercase letters, numbers, and hyphens only)".to_string()
            ));
        }

        match self.repository.claim_task(request.task_id, &request.agent_name).await {
            Ok(task) => {
                // Protocol layer validation: ensure claimed task is in InProgress state
                if task.state != ::task_core::TaskState::InProgress {
                    return Err(McpError::internal(format!(
                        "CRITICAL: claim_task succeeded but task {} is in state {:?}, expected InProgress",
                        task.id, task.state
                    )));
                }

                // Ensure task ownership is correctly set
                if task.owner_agent_name.as_deref() != Some(&request.agent_name) {
                    return Err(McpError::internal(format!(
                        "CRITICAL: claim_task succeeded but task {} owner is {:?}, expected Some('{}')",
                        task.id, task.owner_agent_name, request.agent_name
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

    /// Release a claimed task back to the pool
    #[tool(description = "Release a claimed task back to the pool")]
    async fn release_task(
        &self,
        request: ReleaseTaskRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.repository.release_task(request.task_id, &request.agent_name).await {
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

    /// Start a work session for task tracking
    #[tool(description = "Start a work session for task tracking")]
    async fn start_work_session(
        &self,
        request: StartWorkSessionRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.repository.start_work_session(request.task_id, &request.agent_name).await {
            Ok(session_id) => {
                let session_info = WorkSessionInfo {
                    session_id,
                    task_id: request.task_id,
                    agent_name: request.agent_name,
                    started_at: chrono::Utc::now(),
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&session_info).unwrap(),
                )]))
            }
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// End a work session with productivity metrics
    #[tool(description = "End a work session with productivity metrics")]
    async fn end_work_session(
        &self,
        request: EndWorkSessionRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.repository.end_work_session(
            request.session_id,
            request.notes,
            request.productivity_score,
        ).await {
            Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                r#"{"status": "success", "message": "Work session ended"}"#.to_string(),
            )])),
            Err(TaskError::NotFound(msg)) => Err(McpError::invalid_params(msg)),
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Create a message within a task context
    #[tool(description = "Create a message within a task context for inter-agent communication")]
    async fn create_task_message(
        &self,
        request: CreateTaskMessageRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.message_repository.create_message(
            &request.task_code,
            &request.author_agent_name,
            request.target_agent_name.as_deref(),
            &request.message_type,
            &request.content,
            request.reply_to_message_id,
        ).await {
            Ok(message) => {
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&message).unwrap(),
                )]))
            }
            Err(e) => Err(McpError::internal(format!("Database error: {}", e))),
        }
    }

    /// Get messages from a task with filtering
    #[tool(description = "Get messages from a task with advanced filtering options")]
    async fn get_task_messages(
        &self,
        request: GetTaskMessagesRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.message_repository.get_messages(
            &request.task_code,
            request.author_agent_name.as_deref(),
            request.target_agent_name.as_deref(),
            request.message_type.as_deref(),
            request.reply_to_message_id,
            request.limit,
        ).await {
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
        request: GetSetupInstructionsRequest,
    ) -> Result<CallToolResult, McpError> {
        // Parse AI tool type, default to claude-code if not provided or invalid
        let ai_tool_type = match request.ai_tool_type.as_str() {
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

    /// Get recommended agent workflow for workspace
    #[tool(description = "Get recommended multi-agent workflow description for the workspace")]
    async fn get_agentic_workflow_description(
        &self,
        request: GetAgenticWorkflowDescriptionRequest,
    ) -> Result<CallToolResult, McpError> {
        // Get agent count (default to 3 if not specified)
        let agent_count = request.requested_agent_count.unwrap_or(3);
        
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
        let workflow_description = AgenticWorkflowDescription {
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
            serde_json::to_string_pretty(&workflow_description).unwrap(),
        )]))
    }

    /// Register an AI agent in the workspace
    #[tool(description = "Register an AI agent in the workspace with capabilities and description")]
    async fn register_agent(
        &self,
        request: RegisterAgentRequest,
    ) -> Result<CallToolResult, McpError> {
        use task_core::protocol::DEFAULT_WORKSPACE_ID;
        
        // Basic validation
        if request.agent_name.trim().is_empty() {
            return Err(McpError::invalid_params("Agent name cannot be empty".to_string()));
        }

        if let Some(ref desc) = request.description {
            if desc.len() > 300 {
                return Err(McpError::invalid_params("Agent description cannot exceed 300 characters".to_string()));
            }
        }

        // Validate name format (kebab-case)
        if !request
            .agent_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(McpError::invalid_params(
                "Agent name must be in kebab-case format (lowercase letters, numbers, and hyphens only)".to_string()
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
                .any(|agent| agent.name == request.agent_name)
            {
                return Err(McpError::invalid_params(format!(
                    "Agent with name '{}' already exists",
                    request.agent_name
                )));
            }

            // 4. Construct new AgentRegistration
            let agent_registration = AgentRegistration {
                name: request.agent_name.clone(),
                description: request.description.clone().unwrap_or_default(),
                prompt: format!("Agent: {}, Type: {}", request.agent_name, request.agent_type),
                capabilities: request.capabilities.clone(),
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
                    // Also create agent file in filesystem if project root is available
                    if let Some(_project_root) = &self._project_root {
                        // Create agent file in .claude/agents/ directory
                        // This matches our existing filesystem integration
                        let agent_dir = std::path::Path::new(".claude/agents");
                        if let Err(e) = std::fs::create_dir_all(agent_dir) {
                            tracing::warn!("Failed to create agent directory: {}", e);
                        } else {
                            let agent_file = agent_dir.join(format!("{}.md", request.agent_name));
                            let agent_content = format!(
                                r#"# Agent: {}

## Type
{}

## Description
{}

## Capabilities
{}

## Prompt
{}

---
*Generated by MCP Task Server*
"#,
                                request.agent_name,
                                request.agent_type,
                                request.description.unwrap_or_default(),
                                request.capabilities.join(", "),
                                agent_registration.prompt
                            );
                            
                            if let Err(e) = std::fs::write(&agent_file, agent_content) {
                                tracing::warn!("Failed to write agent file: {}", e);
                            }
                        }
                    }

                    return Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&agent_registration).unwrap(),
                    )]));
                }
                Err(TaskError::DuplicateKey(_)) | Err(TaskError::Conflict(_)) => {
                    // Race condition detected
                    if attempt >= MAX_ATTEMPTS {
                        return Err(McpError::internal(format!(
                            "Workspace concurrently modified after {} attempts; please retry",
                            MAX_ATTEMPTS
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

    /// Get instructions for creating main AI coordination file
    #[tool(description = "Get instructions for creating the main AI coordination file (CLAUDE.md, etc.)")]
    async fn get_instructions_for_main_ai_file(
        &self,
        _request: GetInstructionsForMainAiFileRequest,
    ) -> Result<CallToolResult, McpError> {
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

    /// Create the main AI coordination file
    #[tool(description = "Create the main AI coordination file (CLAUDE.md) with provided content")]
    async fn create_main_ai_file(
        &self,
        request: CreateMainAiFileRequest,
    ) -> Result<CallToolResult, McpError> {
        use task_core::protocol::DEFAULT_WORKSPACE_ID;
        
        // Generate the main AI file (only once – outside the retry loop)
        let response = self
            .workspace_setup_service
            .create_main_file(
                &request.content,
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
                Ok(_) => return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response.payload).unwrap(),
                )])),
                Err(TaskError::DuplicateKey(_)) | Err(TaskError::Conflict(_)) => {
                    // Race condition detected
                    if attempt >= MAX_ATTEMPTS {
                        return Err(McpError::internal(format!(
                            "Workspace concurrently modified after {} attempts; please retry",
                            MAX_ATTEMPTS
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
    > ServerHandler for RmcpTaskHandler<R, M, W>
{
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A comprehensive MCP server for AI task management and multi-agent coordination. Provides 22 functions including task management, agent coordination, messaging, and workspace setup.".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
        }
    }
}