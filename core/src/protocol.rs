use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{
    error::Result,
    models::{Task, TaskFilter, TaskState},
};

/// Protocol handler trait for MCP operations
/// 
/// This trait defines the interface for all MCP protocol operations.
/// Implementations must handle MCP message routing and parameter validation.
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// Create a new task via MCP
    async fn create_task(&self, params: CreateTaskParams) -> Result<Task>;

    /// Update an existing task via MCP
    async fn update_task(&self, params: UpdateTaskParams) -> Result<Task>;

    /// Set task state via MCP
    async fn set_task_state(&self, params: SetStateParams) -> Result<Task>;

    /// Get a task by ID via MCP
    async fn get_task_by_id(&self, params: GetTaskByIdParams) -> Result<Option<Task>>;

    /// Get a task by code via MCP
    async fn get_task_by_code(&self, params: GetTaskByCodeParams) -> Result<Option<Task>>;

    /// List tasks via MCP
    async fn list_tasks(&self, params: ListTasksParams) -> Result<Vec<Task>>;

    /// Assign a task to a different agent via MCP
    async fn assign_task(&self, params: AssignTaskParams) -> Result<Task>;

    /// Archive a task via MCP
    async fn archive_task(&self, params: ArchiveTaskParams) -> Result<Task>;

    /// Handle health check request via MCP
    async fn health_check(&self) -> Result<HealthStatus>;
}

/// MCP parameters for creating a new task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskParams {
    pub code: String,
    pub name: String,
    pub description: String,
    pub owner_agent_name: String,
}

/// MCP parameters for updating a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskParams {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub owner_agent_name: Option<String>,
}

/// MCP parameters for changing task state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetStateParams {
    pub id: i32,
    pub state: TaskState,
}

/// MCP parameters for getting a task by ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskByIdParams {
    pub id: i32,
}

/// MCP parameters for getting a task by code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskByCodeParams {
    pub code: String,
}

/// MCP parameters for listing tasks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListTasksParams {
    pub owner_agent_name: Option<String>,
    pub state: Option<TaskState>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub completed_after: Option<String>,
    pub completed_before: Option<String>,
    pub limit: Option<u32>,
}

/// MCP parameters for assigning a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignTaskParams {
    pub id: i32,
    pub new_owner: String,
}

/// MCP parameters for archiving a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveTaskParams {
    pub id: i32,
}

/// Health status response for MCP clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub database: bool,
    pub protocol: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            status: "unknown".to_string(),
            database: false,
            protocol: false,
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl ListTasksParams {
    /// Convert MCP parameters to internal TaskFilter
    pub fn to_task_filter(&self) -> Result<TaskFilter> {
        use chrono::{DateTime, Utc};
        
        let parse_datetime = |s: &str| -> Result<DateTime<Utc>> {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| crate::error::TaskError::Validation(format!("Invalid datetime format: {e}")))
        };

        let created_after = match &self.created_after {
            Some(s) => Some(parse_datetime(s)?),
            None => None,
        };

        let created_before = match &self.created_before {
            Some(s) => Some(parse_datetime(s)?),
            None => None,
        };

        // TODO: Add support for completion date filtering to TaskFilter
        // Currently unused because TaskFilter doesn't have completed_after/completed_before fields
        let _completed_after = match &self.completed_after {
            Some(s) => Some(parse_datetime(s)?),
            None => None,
        };

        let _completed_before = match &self.completed_before {
            Some(s) => Some(parse_datetime(s)?),
            None => None,
        };

        Ok(TaskFilter {
            owner: self.owner_agent_name.clone(),
            state: self.state,
            date_from: created_after,
            date_to: created_before,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_params_to_filter_conversion() {
        let params = ListTasksParams {
            owner_agent_name: Some("test-agent".to_string()),
            state: Some(TaskState::InProgress),
            created_after: Some("2023-12-01T00:00:00Z".to_string()),
            created_before: Some("2023-12-31T23:59:59Z".to_string()),
            limit: Some(10),
            ..Default::default()
        };

        let filter = params.to_task_filter().unwrap();
        assert_eq!(filter.owner, Some("test-agent".to_string()));
        assert_eq!(filter.state, Some(TaskState::InProgress));
        assert!(filter.date_from.is_some());
        assert!(filter.date_to.is_some());
        // Note: limit is not stored in TaskFilter, it's handled at the protocol layer
    }

    #[test]
    fn test_health_status_default() {
        let health = HealthStatus::default();
        assert_eq!(health.status, "unknown");
        assert!(!health.database);
        assert!(!health.protocol);
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    }
}