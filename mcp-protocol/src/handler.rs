//! MCP Task Handler
//! 
//! Implements the ProtocolHandler trait for MCP communication.

use std::sync::Arc;
use ::task_core::{TaskRepository, TaskMessageRepository, WorkspaceContextRepository, ProtocolHandler, Task, NewTask, HealthStatus, TaskMessage};
use ::task_core::{DiscoverWorkParams, ClaimTaskParams, ReleaseTaskParams, StartWorkSessionParams, EndWorkSessionParams, WorkSessionInfo};
use ::task_core::{CreateTaskMessageParams, GetTaskMessagesParams};
use ::task_core::{
    GetSetupInstructionsParams, GetAgenticWorkflowDescriptionParams, RegisterAgentParams,
    GetInstructionsForMainAiFileParams, CreateMainAiFileParams, GetWorkspaceManifestParams,
    WorkspaceSetupService, SetupInstructions, AgenticWorkflowDescription, 
    AgentRegistration, MainAiFileInstructions, MainAiFileData, WorkspaceManifest, PrdDocument,
};
use ::task_core::error::Result;
use ::task_core::TaskError;
use crate::serialization::*;
use async_trait::async_trait;

// Maximum attempts for get-or-modify loops to handle race conditions
const MAX_ATTEMPTS: u8 = 5;

/// MCP Task Handler that bridges MCP protocol with TaskRepository, TaskMessageRepository, and WorkspaceContextRepository
#[derive(Clone)]
pub struct McpTaskHandler<R, M, W> {
    repository: Arc<R>,
    message_repository: Arc<M>,
    workspace_context_repository: Arc<W>,
    workspace_setup_service: WorkspaceSetupService,
}

impl<R, M, W> McpTaskHandler<R, M, W> {
    /// Create new MCP task handler
    pub fn new(
        repository: Arc<R>, 
        message_repository: Arc<M>, 
        workspace_context_repository: Arc<W>
    ) -> Self {
        Self { 
            repository, 
            message_repository,
            workspace_context_repository: workspace_context_repository.clone(),
            workspace_setup_service: WorkspaceSetupService::new(),
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
impl<R: TaskRepository + Send + Sync, M: TaskMessageRepository + Send + Sync, W: WorkspaceContextRepository + Send + Sync> ProtocolHandler for McpTaskHandler<R, M, W> {
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
        self.repository.update(params.id, params.into_update_data()).await
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
        self.repository.discover_work(&params.agent_name, &params.capabilities, max_tasks).await
    }

    async fn claim_task(&self, params: ClaimTaskParams) -> Result<Task> {
        // Validate agent name format at protocol layer
        if params.agent_name.trim().is_empty() {
            return Err(::task_core::TaskError::Validation("Agent name cannot be empty".to_string()));
        }
        
        // Validate agent name format (kebab-case)
        if !params.agent_name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(::task_core::TaskError::Validation(
                "Agent name must be in kebab-case format (lowercase letters, numbers, and hyphens only)".to_string()
            ));
        }
        
        // Call repository with validated parameters
        let claimed_task = self.repository.claim_task(params.task_id, &params.agent_name).await?;
        
        // Protocol layer validation: ensure claimed task is in InProgress state
        if claimed_task.state != ::task_core::TaskState::InProgress {
            return Err(::task_core::TaskError::Internal(
                format!("CRITICAL: claim_task succeeded but task {} is in state {:?}, expected InProgress", 
                        claimed_task.id, claimed_task.state)
            ));
        }
        
        // Ensure task ownership is correctly set
        if claimed_task.owner_agent_name.as_deref() != Some(&params.agent_name) {
            return Err(::task_core::TaskError::Internal(
                format!("CRITICAL: claim_task succeeded but task {} owner is {:?}, expected Some('{}')", 
                        claimed_task.id, claimed_task.owner_agent_name, params.agent_name)
            ));
        }
        
        Ok(claimed_task)
    }

    async fn release_task(&self, params: ReleaseTaskParams) -> Result<Task> {
        self.repository.release_task(params.task_id, &params.agent_name).await
    }

    async fn start_work_session(&self, params: StartWorkSessionParams) -> Result<WorkSessionInfo> {
        let session_id = self.repository.start_work_session(params.task_id, &params.agent_name).await?;
        Ok(WorkSessionInfo {
            session_id,
            task_id: params.task_id,
            agent_name: params.agent_name,
            started_at: chrono::Utc::now(),
        })
    }

    async fn end_work_session(&self, params: EndWorkSessionParams) -> Result<()> {
        self.repository.end_work_session(params.session_id, params.notes, params.productivity_score).await
    }

    // Task Messaging Implementation
    
    async fn create_task_message(&self, params: CreateTaskMessageParams) -> Result<TaskMessage> {
        self.message_repository.create_message(
            &params.task_code,
            &params.author_agent_name,
            params.target_agent_name.as_deref(),
            &params.message_type,
            &params.content,
            params.reply_to_message_id,
        ).await
    }
    
    async fn get_task_messages(&self, params: GetTaskMessagesParams) -> Result<Vec<TaskMessage>> {
        self.message_repository.get_messages(
            &params.task_code,
            params.author_agent_name.as_deref(),
            params.target_agent_name.as_deref(),
            params.message_type.as_deref(),
            params.reply_to_message_id,
            params.limit,
        ).await
    }

    // Workspace Setup Implementation
    
    async fn get_setup_instructions(&self, params: GetSetupInstructionsParams) -> Result<SetupInstructions> {
        use ::task_core::WorkspaceSetupError;
        
        let response = self.workspace_setup_service
            .get_setup_instructions(params.ai_tool_type)
            .await
            .map_err(|e| match e {
                WorkspaceSetupError::UnsupportedAiTool(tool) => {
                    ::task_core::TaskError::Validation(format!("Unsupported AI tool type: {tool}"))
                }
                WorkspaceSetupError::InvalidConfiguration(msg) => {
                    ::task_core::TaskError::Validation(format!("Invalid configuration: {msg}"))
                }
                _ => ::task_core::TaskError::Protocol(format!("Workspace setup error: {e}")),
            })?;
        
        Ok(response.payload)
    }
    
    async fn get_agentic_workflow_description(&self, params: GetAgenticWorkflowDescriptionParams) -> Result<AgenticWorkflowDescription> {
        use ::task_core::WorkspaceSetupError;
        
        // Parse PRD content
        let prd = PrdDocument::from_content(&params.prd_content)
            .map_err(|e| match e {
                WorkspaceSetupError::PrdParsingFailed(msg) => {
                    ::task_core::TaskError::Validation(format!("PRD parsing failed: {msg}"))
                }
                WorkspaceSetupError::PrdValidationFailed { errors } => {
                    ::task_core::TaskError::Validation(format!("PRD validation failed: {errors:?}"))
                }
                _ => ::task_core::TaskError::Protocol(format!("PRD processing error: {e}")),
            })?;
        
        // Check if PRD is valid
        if !prd.is_valid() {
            return Err(::task_core::TaskError::Validation(
                format!("Invalid PRD: {:?}", prd.get_validation_errors())
            ));
        }
        
        let response = self.workspace_setup_service
            .get_agentic_workflow_description(&prd)
            .await
            .map_err(|e| ::task_core::TaskError::Protocol(format!("Workflow analysis error: {e}")))?;
        
        // Get-or-create workspace context for statefulness
        let maybe_context = self.workspace_context_repository
            .get_by_id(&params.workspace_id)
            .await?;

        let mut workspace_context = maybe_context
            .clone()
            .unwrap_or_else(|| ::task_core::workspace_setup::WorkspaceContext::new(params.workspace_id.clone()));
        
        // Update context with new data
        workspace_context.prd_content = Some(params.prd_content);
        workspace_context.workflow_data = Some(response.payload.clone());
        
        // Save context (create if new, update if existing)
        if maybe_context.is_some() {
            // The context existed, so we must call update
            self.workspace_context_repository
                .update(workspace_context)
                .await
                .map_err(|e| match e {
                    ::task_core::TaskError::Conflict(msg) => ::task_core::TaskError::Conflict(
                        format!("Concurrent modification detected: {msg}. Please retry the operation.")
                    ),
                    _ => ::task_core::TaskError::Database(format!("Failed to update workspace context: {e}"))
                })?;
        } else {
            // The context was new, so we must call create
            self.workspace_context_repository
                .create(workspace_context)
                .await
                .map_err(|e| ::task_core::TaskError::Database(format!("Failed to create workspace context: {e}")))?;
        }
        
        Ok(response.payload)
    }
    
    async fn register_agent(&self, params: RegisterAgentParams) -> Result<AgentRegistration> {
        // Basic validation
        if params.name.trim().is_empty() {
            return Err(::task_core::TaskError::Validation("Agent name cannot be empty".to_string()));
        }
        
        if params.description.len() > 300 {
            return Err(::task_core::TaskError::Validation("Agent description cannot exceed 300 characters".to_string()));
        }
        
        if params.prompt.trim().is_empty() {
            return Err(::task_core::TaskError::Validation("Agent prompt cannot be empty".to_string()));
        }
        
        // Validate name format (kebab-case)
        if !params.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
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
                .get_by_id(&params.workspace_id)
                .await?;

            // 2. Check if context exists and prepare for modification
            let context_exists = maybe_context.is_some();
            let mut workspace_context = maybe_context
                .unwrap_or_else(|| ::task_core::workspace_setup::WorkspaceContext::new(params.workspace_id.clone()));

            // 3. Duplicate-agent guard (in case another peer registered same name first)
            if workspace_context
                .registered_agents
                .iter()
                .any(|agent| agent.name == params.name)
            {
                return Err(TaskError::DuplicateKey(format!(
                    "Agent with name '{}' already exists",
                    params.name
                )));
            }

            // 4. Construct new AgentRegistration
            let agent_registration = AgentRegistration {
                name: params.name.clone(),
                description: params.description.clone(),
                prompt: params.prompt.clone(),
                capabilities: params.capabilities.clone(),
                ai_tool_type: params.ai_tool_type,
                dependencies: params
                    .dependencies
                    .clone()
                    .unwrap_or_default(),
            };

            // 5. Mutate context
            workspace_context.registered_agents.push(agent_registration.clone());
            workspace_context.updated_at = chrono::Utc::now();

            // 6. Persist with get-or-modify pattern
            let write_result = if context_exists {
                self.workspace_context_repository.update(workspace_context).await
            } else {
                self.workspace_context_repository.create(workspace_context).await
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
                    tokio::time::sleep(tokio::time::Duration::from_millis(10 * attempt as u64)).await;
                    continue;
                }
                Err(e) => return Err(e), // any other failure
            }
        }
    }
    
    async fn get_instructions_for_main_ai_file(&self, params: GetInstructionsForMainAiFileParams) -> Result<MainAiFileInstructions> {
        let response = self.workspace_setup_service
            .get_main_file_instructions(params.ai_tool_type)
            .await
            .map_err(|e| ::task_core::TaskError::Protocol(format!("Main AI file instructions error: {e}")))?;
        
        Ok(response.payload)
    }
    
    async fn create_main_ai_file(&self, params: CreateMainAiFileParams) -> Result<MainAiFileData> {
        // Generate the main AI file (only once – outside the retry loop)
        let response = self
            .workspace_setup_service
            .create_main_file(&params.content, params.ai_tool_type, params.project_name.as_deref())
            .await
            .map_err(|e| ::task_core::TaskError::Protocol(format!("Main AI file creation error: {e}")))?;

        // Pre-build the file metadata so it can be reused when we retry
        let file_metadata = ::task_core::workspace_setup::GeneratedFileMetadata {
            path: response.payload.file_name.clone(),
            description: format!("Main AI coordination file for {}", params.ai_tool_type),
            ai_tool_type: params.ai_tool_type,
            content_type: "text/markdown".to_string(),
            created_at: chrono::Utc::now(),
        };

        // Get-or-modify pattern with retry loop for race condition handling
        let mut attempt = 0u8;
        loop {
            // 1. Fetch or create workspace context
            let maybe_context = self
                .workspace_context_repository
                .get_by_id(&params.workspace_id)
                .await?;

            let context_exists = maybe_context.is_some();
            let mut workspace_context = maybe_context
                .unwrap_or_else(|| ::task_core::workspace_setup::WorkspaceContext::new(params.workspace_id.clone()));

            // 2. Avoid duplicate insertion on retry
            if !workspace_context
                .generated_files
                .iter()
                .any(|f| f.path == file_metadata.path)
            {
                workspace_context.generated_files.push(file_metadata.clone());
                workspace_context.updated_at = chrono::Utc::now();
            }

            // 3. Persist
            let write_result = if context_exists {
                self.workspace_context_repository.update(workspace_context).await
            } else {
                self.workspace_context_repository.create(workspace_context).await
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
                    tokio::time::sleep(tokio::time::Duration::from_millis(10 * attempt as u64)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    async fn get_workspace_manifest(&self, params: GetWorkspaceManifestParams) -> Result<WorkspaceManifest> {
        // Load workspace context from repository using workspace_id
        let workspace_context = self.workspace_context_repository
            .get_by_id(&params.workspace_id)
            .await?
            .ok_or_else(|| ::task_core::TaskError::NotFound(format!("Workspace not found: {}", params.workspace_id)))?;
        
        // Parse PRD from workspace context
        let prd_content = workspace_context.prd_content
            .as_ref()
            .ok_or_else(|| ::task_core::TaskError::Validation("Workspace has no PRD content".to_string()))?;
            
        let prd = ::task_core::workspace_setup::PrdDocument::from_content(prd_content)
            .map_err(|e| ::task_core::TaskError::Validation(format!("Failed to parse PRD: {e}")))?;
        
        // Use registered agents from workspace context
        let agents = workspace_context.registered_agents.clone();
        
        let include_generated_files = params.include_generated_files.unwrap_or(true);
        
        let response = self.workspace_setup_service
            .generate_workspace_manifest(&prd, &agents, include_generated_files)
            .await
            .map_err(|e| ::task_core::TaskError::Protocol(format!("Workspace manifest generation error: {e}")))?;
        
        Ok(response.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;
    use ::task_core::{TaskFilter, TaskState, RepositoryStats, UpdateTask};
    
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
        async fn create(&self, _context: ::task_core::workspace_setup::WorkspaceContext) -> Result<::task_core::workspace_setup::WorkspaceContext> {
            unimplemented!()
        }
        
        async fn get_by_id(&self, _workspace_id: &str) -> Result<Option<::task_core::workspace_setup::WorkspaceContext>> {
            Ok(None)
        }
        
        async fn update(&self, _context: ::task_core::workspace_setup::WorkspaceContext) -> Result<::task_core::workspace_setup::WorkspaceContext> {
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
        let _handler = McpTaskHandler::new(mock_repo, mock_message_repo, mock_workspace_repo);
        // Basic test that handler can be created
        assert!(true);
    }
}