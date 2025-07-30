use async_trait::async_trait;
use crate::{
    error::Result,
    models::{Task, TaskFilter, TaskState, NewTask, UpdateTask},
};

/// Repository trait for task persistence and retrieval operations
/// 
/// This trait defines the interface for all task data operations.
/// Implementations must be thread-safe and support concurrent access.
#[async_trait]
pub trait TaskRepository: Send + Sync {
    /// Create a new task
    /// 
    /// # Arguments
    /// * `task` - The new task data to create
    /// 
    /// # Returns
    /// * `Ok(Task)` - The created task with assigned ID and timestamps
    /// * `Err(TaskError::DuplicateCode)` - If the task code already exists
    /// * `Err(TaskError::Validation)` - If the task data is invalid
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn create(&self, task: NewTask) -> Result<Task>;

    /// Update an existing task
    /// 
    /// # Arguments
    /// * `id` - The task ID to update
    /// * `updates` - The fields to update (only non-None fields are updated)
    /// 
    /// # Returns
    /// * `Ok(Task)` - The updated task
    /// * `Err(TaskError::NotFound)` - If the task doesn't exist
    /// * `Err(TaskError::Validation)` - If the update data is invalid
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn update(&self, id: i32, updates: UpdateTask) -> Result<Task>;

    /// Change the state of a task
    /// 
    /// # Arguments
    /// * `id` - The task ID to update
    /// * `state` - The new state to set
    /// 
    /// # Returns
    /// * `Ok(Task)` - The updated task with completion timestamp if moving to Done
    /// * `Err(TaskError::NotFound)` - If the task doesn't exist
    /// * `Err(TaskError::InvalidStateTransition)` - If the state transition is invalid
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn set_state(&self, id: i32, state: TaskState) -> Result<Task>;

    /// Get a task by its numeric ID
    /// 
    /// # Arguments
    /// * `id` - The task ID to find
    /// 
    /// # Returns
    /// * `Ok(Some(Task))` - The task if found
    /// * `Ok(None)` - If no task exists with that ID
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn get_by_id(&self, id: i32) -> Result<Option<Task>>;

    /// Get a task by its human-readable code
    /// 
    /// # Arguments
    /// * `code` - The task code to find (e.g., "ARCH-01")
    /// 
    /// # Returns
    /// * `Ok(Some(Task))` - The task if found
    /// * `Ok(None)` - If no task exists with that code
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn get_by_code(&self, code: &str) -> Result<Option<Task>>;

    /// List tasks matching the given filter criteria
    /// 
    /// # Arguments
    /// * `filter` - The filter criteria to apply
    /// 
    /// # Returns
    /// * `Ok(Vec<Task>)` - The matching tasks (may be empty)
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>>;

    /// Assign a task to a different agent
    /// 
    /// # Arguments
    /// * `id` - The task ID to reassign
    /// * `new_owner` - The new owner agent name
    /// 
    /// # Returns
    /// * `Ok(Task)` - The updated task with new owner
    /// * `Err(TaskError::NotFound)` - If the task doesn't exist
    /// * `Err(TaskError::Validation)` - If the new owner name is invalid
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn assign(&self, id: i32, new_owner: &str) -> Result<Task>;

    /// Archive a task (move to archived state)
    /// 
    /// # Arguments
    /// * `id` - The task ID to archive
    /// 
    /// # Returns
    /// * `Ok(Task)` - The archived task
    /// * `Err(TaskError::NotFound)` - If the task doesn't exist
    /// * `Err(TaskError::InvalidStateTransition)` - If the task cannot be archived from its current state
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn archive(&self, id: i32) -> Result<Task>;

    /// Get repository health status for monitoring
    /// 
    /// # Returns
    /// * `Ok(())` - Repository is healthy and connected
    /// * `Err(TaskError::Database)` - Repository is unhealthy
    async fn health_check(&self) -> Result<()>;

    /// Get repository statistics for monitoring
    /// 
    /// # Returns
    /// * `Ok(RepositoryStats)` - Current repository statistics
    /// * `Err(TaskError::Database)` - If unable to gather statistics
    async fn get_stats(&self) -> Result<RepositoryStats>;

    // MCP v2 Advanced Multi-Agent Features

    /// Discover available work for an agent based on capabilities
    /// 
    /// # Arguments
    /// * `agent_name` - The agent looking for work
    /// * `capabilities` - Agent's capabilities for task matching  
    /// * `max_tasks` - Maximum number of tasks to return
    /// 
    /// # Returns
    /// * `Ok(Vec<Task>)` - Available tasks matching agent capabilities
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn discover_work(&self, agent_name: &str, capabilities: &[String], max_tasks: u32) -> Result<Vec<Task>>;

    /// Claim a task for execution by an agent
    /// 
    /// # Arguments
    /// * `task_id` - The task to claim
    /// * `agent_name` - The agent claiming the task
    /// 
    /// # Returns
    /// * `Ok(Task)` - The claimed task
    /// * `Err(TaskError::NotFound)` - If the task doesn't exist
    /// * `Err(TaskError::AlreadyClaimed)` - If the task is already claimed
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn claim_task(&self, task_id: i32, agent_name: &str) -> Result<Task>;

    /// Release a previously claimed task
    /// 
    /// # Arguments
    /// * `task_id` - The task to release
    /// * `agent_name` - The agent releasing the task
    /// 
    /// # Returns
    /// * `Ok(Task)` - The released task
    /// * `Err(TaskError::NotFound)` - If the task doesn't exist
    /// * `Err(TaskError::NotOwned)` - If the agent doesn't own the task
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn release_task(&self, task_id: i32, agent_name: &str) -> Result<Task>;

    /// Start a work session for time tracking
    /// 
    /// # Arguments
    /// * `task_id` - The task being worked on
    /// * `agent_name` - The agent starting work
    /// 
    /// # Returns
    /// * `Ok(i32)` - The work session ID
    /// * `Err(TaskError::NotFound)` - If the task doesn't exist
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn start_work_session(&self, task_id: i32, agent_name: &str) -> Result<i32>;

    /// End a work session and record productivity
    /// 
    /// # Arguments
    /// * `session_id` - The work session to end
    /// * `notes` - Optional notes about the work done
    /// * `productivity_score` - Productivity score (0.0-1.0)
    /// 
    /// # Returns
    /// * `Ok(())` - Session ended successfully
    /// * `Err(TaskError::NotFound)` - If the session doesn't exist
    /// * `Err(TaskError::Database)` - If the database operation fails
    async fn end_work_session(&self, session_id: i32, notes: Option<String>, productivity_score: Option<f64>) -> Result<()>;
}

/// Repository statistics for monitoring and analytics
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoryStats {
    /// Total number of tasks in the repository
    pub total_tasks: u64,
    /// Number of tasks by state
    pub tasks_by_state: std::collections::HashMap<TaskState, u64>,
    /// Number of tasks by owner agent
    pub tasks_by_owner: std::collections::HashMap<String, u64>,
    /// Most recently created task timestamp
    pub latest_created: Option<chrono::DateTime<chrono::Utc>>,
    /// Most recently completed task timestamp  
    pub latest_completed: Option<chrono::DateTime<chrono::Utc>>,
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_repository_stats_default() {
        let stats = RepositoryStats::default();
        assert_eq!(stats.total_tasks, 0);
        assert!(stats.tasks_by_state.is_empty());
        assert!(stats.tasks_by_owner.is_empty());
        assert!(stats.latest_created.is_none());
        assert!(stats.latest_completed.is_none());
    }

    #[test]
    fn test_repository_stats_creation() {
        let mut stats = RepositoryStats::default();
        stats.total_tasks = 10;
        stats.tasks_by_state.insert(TaskState::InProgress, 5);
        stats.tasks_by_state.insert(TaskState::Done, 3);
        stats.tasks_by_state.insert(TaskState::Created, 2);
        
        stats.tasks_by_owner.insert("agent-1".to_string(), 6);
        stats.tasks_by_owner.insert("agent-2".to_string(), 4);
        
        stats.latest_created = Some(Utc::now());
        stats.latest_completed = Some(Utc::now());

        assert_eq!(stats.total_tasks, 10);
        assert_eq!(stats.tasks_by_state.len(), 3);
        assert_eq!(stats.tasks_by_owner.len(), 2);
        assert!(stats.latest_created.is_some());
        assert!(stats.latest_completed.is_some());
    }
}