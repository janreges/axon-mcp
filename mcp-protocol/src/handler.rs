//! MCP Task Handler
//! 
//! Implements the ProtocolHandler trait for MCP communication.

use std::sync::Arc;
use ::task_core::{TaskRepository, ProtocolHandler, Task, NewTask, UpdateTask, HealthStatus};
use ::task_core::{DiscoverWorkParams, ClaimTaskParams, ReleaseTaskParams, StartWorkSessionParams, EndWorkSessionParams, WorkSessionInfo};
use ::task_core::error::Result;
use crate::serialization::*;
use async_trait::async_trait;

/// MCP Task Handler that bridges MCP protocol with TaskRepository
#[derive(Clone)]
pub struct McpTaskHandler<R> {
    repository: Arc<R>,
}

impl<R> McpTaskHandler<R> {
    /// Create new MCP task handler
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
    
    /// Get a clone of the repository Arc for creating new handlers
    pub fn repository(&self) -> Arc<R> {
        self.repository.clone()
    }
}

#[async_trait]
impl<R: TaskRepository + Send + Sync> ProtocolHandler for McpTaskHandler<R> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;
    use ::task_core::{TaskFilter, TaskState, RepositoryStats};
    
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
    
    #[test]
    fn test_handler_creation() {
        let mock_repo = Arc::new(MockTestRepository::new());
        let _handler = McpTaskHandler::new(mock_repo);
        // Basic test that handler can be created
        assert!(true);
    }
}