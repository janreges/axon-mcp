//! MCP Task Handler
//! 
//! Implements the ProtocolHandler trait for MCP communication.

use std::sync::Arc;
use ::task_core::{TaskRepository, TaskMessageRepository, ProtocolHandler, Task, NewTask, HealthStatus, TaskMessage};
use ::task_core::{DiscoverWorkParams, ClaimTaskParams, ReleaseTaskParams, StartWorkSessionParams, EndWorkSessionParams, WorkSessionInfo};
use ::task_core::{CreateTaskMessageParams, GetTaskMessagesParams};
use ::task_core::{
    GetSetupInstructionsParams, GetAgenticWorkflowDescriptionParams, RegisterAgentParams,
    GetInstructionsForMainAiFileParams, CreateMainAiFileParams, GetWorkspaceManifestParams,
    WorkspaceSetupService, SetupInstructions, AgenticWorkflowDescription, 
    AgentRegistration, MainAiFileInstructions, MainAiFileData, WorkspaceManifest, PrdDocument,
};
use ::task_core::error::Result;
use crate::serialization::*;
use async_trait::async_trait;

/// MCP Task Handler that bridges MCP protocol with TaskRepository and TaskMessageRepository
#[derive(Clone)]
pub struct McpTaskHandler<R, M> {
    repository: Arc<R>,
    message_repository: Arc<M>,
    workspace_setup_service: WorkspaceSetupService,
}

impl<R, M> McpTaskHandler<R, M> {
    /// Create new MCP task handler
    pub fn new(repository: Arc<R>, message_repository: Arc<M>) -> Self {
        Self { 
            repository, 
            message_repository,
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
impl<R: TaskRepository + Send + Sync, M: TaskMessageRepository + Send + Sync> ProtocolHandler for McpTaskHandler<R, M> {
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
        self.repository.claim_task(params.task_id, &params.agent_name).await
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
                    ::task_core::TaskError::Validation(format!("Unsupported AI tool type: {}", tool))
                }
                WorkspaceSetupError::InvalidConfiguration(msg) => {
                    ::task_core::TaskError::Validation(format!("Invalid configuration: {}", msg))
                }
                _ => ::task_core::TaskError::Protocol(format!("Workspace setup error: {}", e)),
            })?;
        
        Ok(response.payload)
    }
    
    async fn get_agentic_workflow_description(&self, params: GetAgenticWorkflowDescriptionParams) -> Result<AgenticWorkflowDescription> {
        use ::task_core::WorkspaceSetupError;
        
        // Parse PRD content
        let prd = PrdDocument::from_content(&params.prd_content)
            .map_err(|e| match e {
                WorkspaceSetupError::PrdParsingFailed(msg) => {
                    ::task_core::TaskError::Validation(format!("PRD parsing failed: {}", msg))
                }
                WorkspaceSetupError::PrdValidationFailed { errors } => {
                    ::task_core::TaskError::Validation(format!("PRD validation failed: {:?}", errors))
                }
                _ => ::task_core::TaskError::Protocol(format!("PRD processing error: {}", e)),
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
            .map_err(|e| ::task_core::TaskError::Protocol(format!("Workflow analysis error: {}", e)))?;
        
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
        
        // Create agent registration
        Ok(AgentRegistration {
            name: params.name,
            description: params.description,
            prompt: params.prompt,
            capabilities: params.capabilities,
            ai_tool_type: params.ai_tool_type,
            dependencies: params.dependencies.unwrap_or_default(),
        })
    }
    
    async fn get_instructions_for_main_ai_file(&self, params: GetInstructionsForMainAiFileParams) -> Result<MainAiFileInstructions> {
        let response = self.workspace_setup_service
            .get_main_file_instructions(params.ai_tool_type)
            .await
            .map_err(|e| ::task_core::TaskError::Protocol(format!("Main AI file instructions error: {}", e)))?;
        
        Ok(response.payload)
    }
    
    async fn create_main_ai_file(&self, params: CreateMainAiFileParams) -> Result<MainAiFileData> {
        let response = self.workspace_setup_service
            .create_main_file(&params.content, params.ai_tool_type, params.project_name.as_deref())
            .await
            .map_err(|e| ::task_core::TaskError::Protocol(format!("Main AI file creation error: {}", e)))?;
        
        Ok(response.payload)
    }
    
    async fn get_workspace_manifest(&self, params: GetWorkspaceManifestParams) -> Result<WorkspaceManifest> {
        // Create a basic PRD for demonstration
        let basic_prd = ::task_core::workspace_setup::PrdDocument::from_content(
            "# Sample Project\n\nA basic project for demonstration.\n\n## Requirements\n\nBasic functionality required."
        ).map_err(|e| ::task_core::TaskError::Validation(format!("Failed to create basic PRD: {}", e)))?;
        
        // Create empty agents list for now
        let agents = vec![];
        let include_generated_files = params.include_generated_files.unwrap_or(true);
        
        let response = self.workspace_setup_service
            .generate_workspace_manifest(&basic_prd, &agents, include_generated_files)
            .await
            .map_err(|e| ::task_core::TaskError::Protocol(format!("Workspace manifest generation error: {}", e)))?;
        
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
    
    #[test]
    fn test_handler_creation() {
        let mock_repo = Arc::new(MockTestRepository::new());
        let mock_message_repo = Arc::new(SimpleTestMessageRepository);
        let _handler = McpTaskHandler::new(mock_repo, mock_message_repo);
        // Basic test that handler can be created
        assert!(true);
    }
}