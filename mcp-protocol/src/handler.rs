//! MCP Task Handler
//!
//! Implements the ProtocolHandler trait for MCP communication.

use crate::serialization::*;
use ::task_core::error::Result;
use ::task_core::TaskError;
use ::task_core::{
    AgentRegistration, AgenticWorkflowDescription, CreateMainAiFileParams,
    GetAgenticWorkflowDescriptionParams, GetInstructionsForMainAiFileParams,
    GetSetupInstructionsParams, MainAiFileData, MainAiFileInstructions,
    RegisterAgentParams, SetupInstructions, WorkspaceSetupService,
};
use ::task_core::{
    ClaimTaskParams, CleanupTimedOutTasksParams, DiscoverWorkParams, EndWorkSessionParams, 
    ReleaseTaskParams, StartWorkSessionParams, WorkSessionInfo,
};
use ::task_core::{CreateTaskMessageParams, GetTaskMessagesParams};
use ::task_core::{
    HealthStatus, NewTask, ProtocolHandler, Task, TaskMessage, TaskMessageRepository,
    TaskRepository, WorkspaceContextRepository,
};
use async_trait::async_trait;
use std::sync::Arc;

// Maximum attempts for get-or-modify loops to handle race conditions
const MAX_ATTEMPTS: u8 = 5;

/// MCP Task Handler that bridges MCP protocol with TaskRepository, TaskMessageRepository, and WorkspaceContextRepository
#[derive(Clone)]
pub struct McpTaskHandler<R, M, W> {
    repository: Arc<R>,
    message_repository: Arc<M>,
    workspace_context_repository: Arc<W>,
    workspace_setup_service: WorkspaceSetupService,
    _project_root: Option<std::path::PathBuf>,
}

impl<R, M, W> McpTaskHandler<R, M, W> {
    /// Create new MCP task handler
    pub fn new(
        repository: Arc<R>,
        message_repository: Arc<M>,
        workspace_context_repository: Arc<W>,
        _project_root: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            repository,
            message_repository,
            workspace_context_repository: workspace_context_repository.clone(),
            workspace_setup_service: WorkspaceSetupService::new(),
            _project_root: _project_root,
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

#[async_trait]
impl<
        R: TaskRepository + Send + Sync,
        M: TaskMessageRepository + Send + Sync,
        W: WorkspaceContextRepository + Send + Sync,
    > ProtocolHandler for McpTaskHandler<R, M, W>
{
    async fn create_task(&self, params: CreateTaskParams) -> Result<Task> {
        let new_task = NewTask::new(
            params.code,
            params.name,
            params.description,
            params.owner_agent_name,
        );

        self.repository.create(new_task).await
    }

    async fn update_task(&self, params: UpdateTaskParams) -> Result<Task> {
        self.repository
            .update(params.id, params.into_update_data())
            .await
    }

    async fn set_task_state(&self, params: SetStateParams) -> Result<Task> {
        self.repository.set_state(params.id, params.state).await
    }

    async fn get_task_by_id(&self, params: GetTaskByIdParams) -> Result<Option<Task>> {
        self.repository.get_by_id(params.id).await
    }

    async fn get_task_by_code(&self, params: GetTaskByCodeParams) -> Result<Option<Task>> {
        self.repository.get_by_code(&params.code).await
    }

    async fn list_tasks(&self, params: ListTasksParams) -> Result<Vec<Task>> {
        let filter = params.to_task_filter()?;

        // Pagination is now handled at the database level for performance
        self.repository.list(filter).await
    }

    async fn assign_task(&self, params: AssignTaskParams) -> Result<Task> {
        self.repository.assign(params.id, &params.new_owner).await
    }

    async fn archive_task(&self, params: ArchiveTaskParams) -> Result<Task> {
        self.repository.archive(params.id).await
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        self.repository.health_check().await?;

        let health = HealthStatus {
            status: "healthy".to_string(),
            database: true,
            protocol: true,
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        Ok(health)
    }

    // MCP v2 Advanced Multi-Agent Features

    async fn discover_work(&self, params: DiscoverWorkParams) -> Result<Vec<Task>> {
        let max_tasks = params.max_tasks.unwrap_or(10); // Default to 10 tasks if not specified
        self.repository
            .discover_work(&params.agent_name, &params.capabilities, max_tasks)
            .await
    }

    async fn claim_task(&self, params: ClaimTaskParams) -> Result<Task> {
        // Validate agent name format at protocol layer
        if params.agent_name.trim().is_empty() {
            return Err(::task_core::TaskError::Validation(
                "Agent name cannot be empty".to_string(),
            ));
        }

        // Validate agent name format (kebab-case)
        if !params
            .agent_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(::task_core::TaskError::Validation(
                "Agent name must be in kebab-case format (lowercase letters, numbers, and hyphens only)".to_string()
            ));
        }

        // Call repository with validated parameters
        let claimed_task = self
            .repository
            .claim_task(params.task_id, &params.agent_name)
            .await?;

        // Protocol layer validation: ensure claimed task is in InProgress state
        if claimed_task.state != ::task_core::TaskState::InProgress {
            return Err(::task_core::TaskError::Internal(format!(
                "CRITICAL: claim_task succeeded but task {} is in state {:?}, expected InProgress",
                claimed_task.id, claimed_task.state
            )));
        }

        // Ensure task ownership is correctly set
        if claimed_task.owner_agent_name.as_deref() != Some(&params.agent_name) {
            return Err(::task_core::TaskError::Internal(format!(
                "CRITICAL: claim_task succeeded but task {} owner is {:?}, expected Some('{}')",
                claimed_task.id, claimed_task.owner_agent_name, params.agent_name
            )));
        }

        Ok(claimed_task)
    }

    async fn release_task(&self, params: ReleaseTaskParams) -> Result<Task> {
        self.repository
            .release_task(params.task_id, &params.agent_name)
            .await
    }

    async fn start_work_session(&self, params: StartWorkSessionParams) -> Result<WorkSessionInfo> {
        let session_id = self
            .repository
            .start_work_session(params.task_id, &params.agent_name)
            .await?;
        Ok(WorkSessionInfo {
            session_id,
            task_id: params.task_id,
            agent_name: params.agent_name,
            started_at: chrono::Utc::now(),
        })
    }

    async fn end_work_session(&self, params: EndWorkSessionParams) -> Result<()> {
        self.repository
            .end_work_session(params.session_id, params.notes, params.productivity_score)
            .await
    }

    async fn cleanup_timed_out_tasks(&self, params: CleanupTimedOutTasksParams) -> Result<Vec<Task>> {
        self.repository
            .cleanup_timed_out_tasks(params.timeout_minutes)
            .await
    }

    // Task Messaging Implementation

    async fn create_task_message(&self, params: CreateTaskMessageParams) -> Result<TaskMessage> {
        self.message_repository
            .create_message(
                &params.task_code,
                &params.author_agent_name,
                params.target_agent_name.as_deref(),
                &params.message_type,
                &params.content,
                params.reply_to_message_id,
            )
            .await
    }

    async fn get_task_messages(&self, params: GetTaskMessagesParams) -> Result<Vec<TaskMessage>> {
        self.message_repository
            .get_messages(
                &params.task_code,
                params.author_agent_name.as_deref(),
                params.target_agent_name.as_deref(),
                params.message_type.as_deref(),
                params.reply_to_message_id,
                params.limit,
            )
            .await
    }

    // Workspace Setup Implementation

    async fn get_setup_instructions(
        &self,
        params: GetSetupInstructionsParams,
    ) -> Result<SetupInstructions> {
        // Parse AI tool type, default to claude-code if not provided or invalid
        let ai_tool_type = match params.ai_tool_type.as_str() {
            "claude-code" => ::task_core::workspace_setup::AiToolType::ClaudeCode,
            _ => ::task_core::workspace_setup::AiToolType::ClaudeCode, // Default fallback
        };

        // Return static setup instructions based on AI tool type
        let response = self
            .workspace_setup_service
            .get_setup_instructions(ai_tool_type)
            .await
            .map_err(|e| ::task_core::TaskError::Protocol(format!("Workspace setup error: {e}")))?;

        Ok(response.payload)
    }

    async fn get_agentic_workflow_description(
        &self,
        params: GetAgenticWorkflowDescriptionParams,
    ) -> Result<AgenticWorkflowDescription> {
        // Get agent count (default to 3 if not specified)
        let agent_count = params.requested_agent_count.unwrap_or(3);
        
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
        Ok(AgenticWorkflowDescription {
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
        })
    }

    async fn register_agent(&self, params: RegisterAgentParams) -> Result<AgentRegistration> {
        use task_core::protocol::DEFAULT_WORKSPACE_ID;
        
        // Basic validation
        if params.agent_name.trim().is_empty() {
            return Err(::task_core::TaskError::Validation(
                "Agent name cannot be empty".to_string(),
            ));
        }

        if let Some(ref desc) = params.description {
            if desc.len() > 300 {
                return Err(::task_core::TaskError::Validation(
                    "Agent description cannot exceed 300 characters".to_string(),
                ));
            }
        }

        // Validate name format (kebab-case)
        if !params
            .agent_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(::task_core::TaskError::Validation(
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
                .await?;

            // 2. Check if context exists and prepare for modification
            let context_exists = maybe_context.is_some();
            let mut workspace_context = maybe_context.unwrap_or_else(|| {
                ::task_core::workspace_setup::WorkspaceContext::new(DEFAULT_WORKSPACE_ID.to_string())
            });

            // 3. Duplicate-agent guard (in case another peer registered same name first)
            if workspace_context
                .registered_agents
                .iter()
                .any(|agent| agent.name == params.agent_name)
            {
                return Err(TaskError::DuplicateKey(format!(
                    "Agent with name '{}' already exists",
                    params.agent_name
                )));
            }

            // 4. Construct new AgentRegistration
            let agent_registration = AgentRegistration {
                name: params.agent_name.clone(),
                description: params.description.clone().unwrap_or_default(),
                prompt: format!("Agent: {}, Type: {}", params.agent_name, params.agent_type),
                capabilities: params.capabilities.clone(),
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
                Ok(_) => return Ok(agent_registration), // success
                Err(TaskError::DuplicateKey(_)) | Err(TaskError::Conflict(_)) => {
                    // Race condition detected
                    if attempt >= MAX_ATTEMPTS {
                        return Err(TaskError::DuplicateKey(format!(
                            "Workspace concurrently modified after {MAX_ATTEMPTS} attempts; please retry"
                        )));
                    }
                    attempt += 1;
                    // Small exponential back-off to reduce contention
                    tokio::time::sleep(tokio::time::Duration::from_millis(10 * attempt as u64))
                        .await;
                    continue;
                }
                Err(e) => return Err(e), // any other failure
            }
        }
    }

    async fn get_instructions_for_main_ai_file(
        &self,
        _params: GetInstructionsForMainAiFileParams,
    ) -> Result<MainAiFileInstructions> {
        // Use ClaudeCode as default AI tool type, ignore file_type for now
        let response = self
            .workspace_setup_service
            .get_main_file_instructions(::task_core::workspace_setup::AiToolType::ClaudeCode)
            .await
            .map_err(|e| {
                ::task_core::TaskError::Protocol(format!("Main AI file instructions error: {e}"))
            })?;

        Ok(response.payload)
    }

    async fn create_main_ai_file(&self, params: CreateMainAiFileParams) -> Result<MainAiFileData> {
        use task_core::protocol::DEFAULT_WORKSPACE_ID;
        
        // Generate the main AI file (only once – outside the retry loop)
        let response = self
            .workspace_setup_service
            .create_main_file(
                &params.content,
                ::task_core::workspace_setup::AiToolType::ClaudeCode,
                None, // No project name provided in simplified interface
            )
            .await
            .map_err(|e| {
                ::task_core::TaskError::Protocol(format!("Main AI file creation error: {e}"))
            })?;

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
                .await?;

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
                Ok(_) => return Ok(response.payload.clone()),
                Err(TaskError::DuplicateKey(_)) | Err(TaskError::Conflict(_)) => {
                    // Race condition detected
                    if attempt >= MAX_ATTEMPTS {
                        return Err(TaskError::Conflict(format!(
                            "Workspace concurrently modified after {MAX_ATTEMPTS} attempts; please retry"
                        )));
                    }
                    attempt += 1;
                    // Small exponential back-off to reduce contention
                    tokio::time::sleep(tokio::time::Duration::from_millis(10 * attempt as u64))
                        .await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use ::task_core::{RepositoryStats, TaskFilter, TaskState, UpdateTask};
    use mockall::mock;
    use mockall::predicate::*;

    mock! {
        TestRepository {}

        #[async_trait]
        impl TaskRepository for TestRepository {
            async fn create(&self, task: NewTask) -> Result<Task>;
            async fn update(&self, id: i32, updates: UpdateTask) -> Result<Task>;
            async fn set_state(&self, id: i32, state: TaskState) -> Result<Task>;
            async fn get_by_id(&self, id: i32) -> Result<Option<Task>>;
            async fn get_by_code(&self, code: &str) -> Result<Option<Task>>;
            async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>>;
            async fn assign(&self, id: i32, new_owner: &str) -> Result<Task>;
            async fn archive(&self, id: i32) -> Result<Task>;
            async fn health_check(&self) -> Result<()>;
            async fn get_stats(&self) -> Result<RepositoryStats>;
            async fn discover_work(&self, agent_name: &str, capabilities: &[String], max_tasks: u32) -> Result<Vec<Task>>;
            async fn claim_task(&self, task_id: i32, agent_name: &str) -> Result<Task>;
            async fn release_task(&self, task_id: i32, agent_name: &str) -> Result<Task>;
            async fn start_work_session(&self, task_id: i32, agent_name: &str) -> Result<i32>;
            async fn end_work_session(&self, session_id: i32, notes: Option<String>, productivity_score: Option<f64>) -> Result<()>;
            async fn cleanup_timed_out_tasks(&self, timeout_minutes: i64) -> Result<Vec<Task>>;
        }
    }

    // Simple mock that implements both traits for testing
    struct SimpleTestMessageRepository;

    #[async_trait]
    impl TaskMessageRepository for SimpleTestMessageRepository {
        async fn create_message(
            &self,
            task_code: &str,
            author_agent_name: &str,
            _target_agent_name: Option<&str>,
            message_type: &str,
            content: &str,
            reply_to_message_id: Option<i32>,
        ) -> Result<TaskMessage> {
            Ok(TaskMessage {
                id: 1,
                task_code: task_code.to_string(),
                author_agent_name: author_agent_name.to_string(),
                target_agent_name: None,
                message_type: message_type.to_string(),
                created_at: chrono::Utc::now(),
                content: content.to_string(),
                reply_to_message_id,
            })
        }

        async fn get_messages(
            &self,
            _task_code: &str,
            _author_agent_name: Option<&str>,
            _target_agent_name: Option<&str>,
            _message_type: Option<&str>,
            _reply_to_message_id: Option<i32>,
            _limit: Option<u32>,
        ) -> Result<Vec<TaskMessage>> {
            Ok(vec![])
        }

        async fn get_message_by_id(&self, _message_id: i32) -> Result<Option<TaskMessage>> {
            Ok(None)
        }
    }

    // Simple mock workspace context repository for testing
    struct SimpleTestWorkspaceContextRepository;

    #[async_trait]
    impl WorkspaceContextRepository for SimpleTestWorkspaceContextRepository {
        async fn create(
            &self,
            _context: ::task_core::workspace_setup::WorkspaceContext,
        ) -> Result<::task_core::workspace_setup::WorkspaceContext> {
            unimplemented!()
        }

        async fn get_by_id(
            &self,
            _workspace_id: &str,
        ) -> Result<Option<::task_core::workspace_setup::WorkspaceContext>> {
            Ok(None)
        }

        async fn update(
            &self,
            _context: ::task_core::workspace_setup::WorkspaceContext,
        ) -> Result<::task_core::workspace_setup::WorkspaceContext> {
            unimplemented!()
        }

        async fn delete(&self, _workspace_id: &str) -> Result<()> {
            Ok(())
        }

        async fn health_check(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_handler_creation() {
        let mock_repo = Arc::new(MockTestRepository::new());
        let mock_message_repo = Arc::new(SimpleTestMessageRepository);
        let mock_workspace_repo = Arc::new(SimpleTestWorkspaceContextRepository);
        let _handler = McpTaskHandler::new(mock_repo, mock_message_repo, mock_workspace_repo, None);
        // Basic test that handler can be created
        // Test passes if handler creation doesn't panic
    }
}
