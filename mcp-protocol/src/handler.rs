//! MCP Task Handler
//! 
//! Implements the ProtocolHandler trait for MCP communication.

use std::sync::Arc;
use ::task_core::{TaskRepository, ProtocolHandler, Task, NewTask, UpdateTask, HealthStatus};
use ::task_core::error::Result;
use crate::serialization::*;
use async_trait::async_trait;

/// MCP Task Handler that bridges MCP protocol with TaskRepository
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
        let new_task = NewTask {
            code: params.code,
            name: params.name,
            description: params.description,
            owner_agent_name: params.owner_agent_name,
        };
        
        self.repository.create(new_task).await
    }
    
    async fn update_task(&self, params: UpdateTaskParams) -> Result<Task> {
        let updates = UpdateTask {
            name: params.name,
            description: params.description,
            owner_agent_name: params.owner_agent_name,
        };
        
        self.repository.update(params.id, updates).await
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