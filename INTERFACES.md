# Shared Interfaces - MCP Task Management Server

## APPEND-ONLY FILE - NEVER OVERWRITE, ONLY APPEND WITH >>

This file contains finalized interfaces that multiple crates depend on.

### Interface Codes:
- `[INTERFACE-TASK-REPOSITORY]` - TaskRepository trait
- `[INTERFACE-PROTOCOL-HANDLER]` - ProtocolHandler trait  
- `[INTERFACE-TASK-MODEL]` - Task struct
- `[INTERFACE-ERROR-TYPES]` - Error types

### Format:
```
[INTERFACE-CODE] TIMESTAMP AGENT: Description
--- BEGIN DEFINITION ---
<interface code>
--- END DEFINITION ---
```

---
## Interface Definitions (newest at bottom)[INTERFACE-TASK-REPOSITORY] 2025-07-29 15:14:15 rust-architect: TASK-REPOSITORY trait ready
--- BEGIN DEFINITION ---
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
}--- END DEFINITION ---
[INTERFACE-PROTOCOL-HANDLER] 2025-07-29 15:14:30 rust-architect: PROTOCOL-HANDLER trait ready
--- BEGIN DEFINITION ---
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
}--- END DEFINITION ---
[INTERFACE-TASK-MODEL] 2025-07-29 15:14:38 rust-architect: TASK-MODEL trait ready
--- BEGIN DEFINITION ---
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Core task representation in the MCP Task Management System.
/// 
/// A task represents a unit of work that can be tracked through its lifecycle,
/// assigned to agents, and managed via the MCP protocol. Each task has a unique
/// numeric ID and human-readable code for easy reference.
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::models::{Task, TaskState};
/// use chrono::Utc;
/// 
/// let task = Task {
///     id: 42,
///     code: "FEAT-001".to_string(),
///     name: "Implement user authentication".to_string(),
///     description: "Add JWT-based auth with role-based access control".to_string(),
///     owner_agent_name: "backend-developer".to_string(),
///     state: TaskState::Created,
///     inserted_at: Utc::now(),
///     done_at: None,
/// };
/// 
/// // Check if task can transition to InProgress
/// assert!(task.can_transition_to(TaskState::InProgress));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    /// Auto-increment primary key
    pub id: i32,
    /// Human-readable identifier (e.g., "ARCH-01", "DB-15")
    pub code: String,
    /// Brief task title
    pub name: String,
    /// Detailed task requirements
    pub description: String,
    /// Assigned agent identifier
    pub owner_agent_name: String,
    /// Current lifecycle state
    pub state: TaskState,
    /// Creation timestamp
    pub inserted_at: DateTime<Utc>,
    /// Completion timestamp
    pub done_at: Option<DateTime<Utc>>,
}

/// Task lifecycle states defining the progression of work.
/// 
/// Tasks move through a defined state machine with validated transitions.
/// The typical flow is: Created → InProgress → Review → Done → Archived,
/// with Blocked as a temporary state that can occur during InProgress.
/// 
/// # State Transitions
/// 
/// - `Created` → `InProgress`
/// - `InProgress` → `Blocked`, `Review`, `Done`  
/// - `Blocked` → `InProgress`
/// - `Review` → `InProgress`, `Done`
/// - `Done` → `Archived` (via archive_task only)
/// - `Archived` → (no transitions allowed)
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::models::{Task, TaskState};
/// use chrono::Utc;
/// 
/// let task = Task {
///     id: 1,
///     code: "TEST-01".to_string(),
///     name: "Test Task".to_string(),
///     description: "A test task".to_string(),
///     owner_agent_name: "test-agent".to_string(),
///     state: TaskState::Created,
///     inserted_at: Utc::now(),
///     done_at: None,
/// };
/// 
/// // Check valid transitions
/// if task.can_transition_to(TaskState::InProgress) {
///     // Safe to move to InProgress
/// }
/// ```
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState {
    /// Newly created task
    Created,
    /// Task is actively being worked on
    InProgress,
    /// Task is blocked and cannot proceed
    Blocked,
    /// Task is ready for review
    Review,
    /// Task has been completed
    Done,
    /// Task has been archived
    Archived,
}

/// Data transfer object for creating new tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    /// Human-readable identifier (e.g., "ARCH-01", "DB-15")
    pub code: String,
    /// Brief task title
    pub name: String,
    /// Detailed task requirements
    pub description: String,
    /// Assigned agent identifier
    pub owner_agent_name: String,
}

/// Data transfer object for updating existing tasks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTask {
    /// Optional new task name
    pub name: Option<String>,
    /// Optional new task description
    pub description: Option<String>,
    /// Optional new owner agent
    pub owner_agent_name: Option<String>,
}

/// Filter criteria for querying tasks.
///
/// All fields are optional to support flexible querying.
/// When multiple fields are specified, they are combined with AND logic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    /// Filter by owner agent name
    pub owner: Option<String>,
    
    /// Filter by task state
    pub state: Option<TaskState>,
    
    /// Filter tasks created on or after this date
    pub date_from: Option<DateTime<Utc>>,
    
    /// Filter tasks created on or before this date
    pub date_to: Option<DateTime<Utc>>,
}

impl Task {
    /// Check if the task can transition to the given state
    pub fn can_transition_to(&self, new_state: TaskState) -> bool {
        use TaskState::*;
        
        match (self.state, new_state) {
            // Can't transition to the same state
            (current, new) if current == new => false,
            
            // Valid transitions from Created
            (Created, InProgress) => true,
            
            // Valid transitions from InProgress
            (InProgress, Blocked | Review | Done) => true,
            
            // Valid transitions from Blocked
            (Blocked, InProgress) => true,
            
            // Valid transitions from Review
            (Review, InProgress | Done) => true,
            
            // Valid transitions from Done
            (Done, Archived) => true,
            
            // No valid transitions from Archived
            (Archived, _) => false,
            
            // All other transitions are invalid
            _ => false,
        }
    }
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::Created => write!(f, "Created"),
            TaskState::InProgress => write!(f, "InProgress"), 
            TaskState::Blocked => write!(f, "Blocked"),
            TaskState::Review => write!(f, "Review"),
            TaskState::Done => write!(f, "Done"),
            TaskState::Archived => write!(f, "Archived"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_state_transitions() {
        let task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        };

        // Created -> InProgress
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Archived));

        // InProgress -> Blocked, Review, Done
        let mut task = task;
        task.state = TaskState::InProgress;
        assert!(task.can_transition_to(TaskState::Blocked));
        assert!(task.can_transition_to(TaskState::Review));
        assert!(task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Blocked -> InProgress
        task.state = TaskState::Blocked;
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Review -> InProgress, Done
        task.state = TaskState::Review;
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Done -> Archived
        task.state = TaskState::Done;
        assert!(task.can_transition_to(TaskState::Archived));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));

        // Archived -> nothing
        task.state = TaskState::Archived;
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
    }

    #[test]
    fn test_no_same_state_transition() {
        let task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::InProgress,
            inserted_at: Utc::now(),
            done_at: None,
        };

        // Cannot transition to the same state
        assert!(!task.can_transition_to(TaskState::InProgress));
    }
}--- END DEFINITION ---
[INTERFACE-ERROR-TYPES] 2025-07-29 15:14:46 rust-architect: ERROR-TYPES trait ready
--- BEGIN DEFINITION ---
use thiserror::Error;
use crate::models::TaskState;

/// Result type alias for task operations
pub type Result<T> = std::result::Result<T, TaskError>;

/// Comprehensive error types for the MCP Task Management System.
/// 
/// These errors cover all possible failure modes in task operations,
/// from validation failures to database errors. Each error type maps
/// to appropriate HTTP status codes for API responses.
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::error::{TaskError, Result};
/// use task_core::models::TaskState;
/// 
/// // Create specific error types
/// let not_found = TaskError::not_found_id(42);
/// let invalid_transition = TaskError::invalid_transition(
///     TaskState::Created, 
///     TaskState::Done
/// );
/// 
/// // Check error categories
/// assert!(not_found.is_not_found());
/// assert_eq!(not_found.status_code(), 404);
/// assert_eq!(invalid_transition.status_code(), 422);
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    /// Task not found by the given identifier
    #[error("Task not found: {0}")]
    NotFound(String),

    /// Invalid state transition attempted
    #[error("Invalid state transition from {0} to {1}")]
    InvalidStateTransition(TaskState, TaskState),

    /// Duplicate task code already exists
    #[error("Task code already exists: {0}")]
    DuplicateCode(String),

    /// Validation error with details
    #[error("Validation error: {0}")]
    Validation(String),

    /// Database operation error
    #[error("Database error: {0}")]
    Database(String),

    /// Protocol error from MCP operations
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Internal system error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl TaskError {
    /// Create a not found error for a task ID
    pub fn not_found_id(id: i32) -> Self {
        Self::NotFound(format!("Task with ID {id} not found"))
    }

    /// Create a not found error for a task code
    pub fn not_found_code(code: &str) -> Self {
        Self::NotFound(format!("Task with code '{code}' not found"))
    }

    /// Create a validation error for invalid task code format
    pub fn invalid_code_format(code: &str) -> Self {
        Self::Validation(format!("Invalid task code format: '{code}'"))
    }

    /// Create a validation error for invalid agent name
    pub fn invalid_agent_name(name: &str) -> Self {
        Self::Validation(format!("Invalid agent name: '{name}'"))
    }

    /// Create a validation error for empty field
    pub fn empty_field(field: &str) -> Self {
        Self::Validation(format!("Field '{field}' cannot be empty"))
    }

    /// Create a state transition error
    pub fn invalid_transition(from: TaskState, to: TaskState) -> Self {
        Self::InvalidStateTransition(from, to)
    }

    /// Check if this error indicates a not found condition
    pub fn is_not_found(&self) -> bool {
        matches!(self, TaskError::NotFound(_))
    }

    /// Check if this error indicates a validation problem
    pub fn is_validation(&self) -> bool {
        matches!(self, TaskError::Validation(_))
    }

    /// Check if this error indicates a database problem
    pub fn is_database(&self) -> bool {
        matches!(self, TaskError::Database(_))
    }

    /// Convert to appropriate HTTP status code equivalent
    pub fn status_code(&self) -> u16 {
        match self {
            TaskError::NotFound(_) => 404,
            TaskError::Validation(_) => 400,
            TaskError::DuplicateCode(_) => 409,
            TaskError::InvalidStateTransition(_, _) => 422,
            TaskError::Database(_) => 500,
            TaskError::Protocol(_) => 500,
            TaskError::Configuration(_) => 500,
            TaskError::Internal(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = TaskError::not_found_id(42);
        assert_eq!(error, TaskError::NotFound("Task with ID 42 not found".to_string()));
        assert!(error.is_not_found());
        assert_eq!(error.status_code(), 404);

        let error = TaskError::not_found_code("ARCH-01");
        assert_eq!(error, TaskError::NotFound("Task with code 'ARCH-01' not found".to_string()));
        
        let error = TaskError::invalid_code_format("invalid-code");
        assert!(error.is_validation());
        assert_eq!(error.status_code(), 400);

        let error = TaskError::invalid_transition(TaskState::Created, TaskState::Done);
        assert_eq!(error, TaskError::InvalidStateTransition(TaskState::Created, TaskState::Done));
        assert_eq!(error.status_code(), 422);
    }

    #[test]
    fn test_error_display() {
        let error = TaskError::NotFound("Task not found".to_string());
        assert_eq!(format!("{}", error), "Task not found: Task not found");

        let error = TaskError::InvalidStateTransition(TaskState::Created, TaskState::Done);
        assert_eq!(format!("{}", error), "Invalid state transition from Created to Done");

        let error = TaskError::Validation("Invalid input".to_string());
        assert_eq!(format!("{}", error), "Validation error: Invalid input");
    }

    #[test]
    fn test_error_predicates() {
        assert!(TaskError::NotFound("test".to_string()).is_not_found());
        assert!(!TaskError::Validation("test".to_string()).is_not_found());

        assert!(TaskError::Validation("test".to_string()).is_validation());
        assert!(!TaskError::Database("test".to_string()).is_validation());

        assert!(TaskError::Database("test".to_string()).is_database());
        assert!(!TaskError::Protocol("test".to_string()).is_database());
    }
}--- END DEFINITION ---
[INTERFACE-TASK-REPOSITORY] 2025-07-29 15:41:25 rust-architect: TASK-REPOSITORY trait ready
--- BEGIN DEFINITION ---
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
}--- END DEFINITION ---
[INTERFACE-PROTOCOL-HANDLER] 2025-07-29 15:41:30 rust-architect: PROTOCOL-HANDLER trait ready
--- BEGIN DEFINITION ---
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
}--- END DEFINITION ---
[INTERFACE-TASK-MODEL] 2025-07-29 15:41:34 rust-architect: TASK-MODEL trait ready
--- BEGIN DEFINITION ---
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Core task representation in the MCP Task Management System.
/// 
/// A task represents a unit of work that can be tracked through its lifecycle,
/// assigned to agents, and managed via the MCP protocol. Each task has a unique
/// numeric ID and human-readable code for easy reference.
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::models::{Task, TaskState};
/// use chrono::Utc;
/// 
/// let task = Task {
///     id: 42,
///     code: "FEAT-001".to_string(),
///     name: "Implement user authentication".to_string(),
///     description: "Add JWT-based auth with role-based access control".to_string(),
///     owner_agent_name: "backend-developer".to_string(),
///     state: TaskState::Created,
///     inserted_at: Utc::now(),
///     done_at: None,
/// };
/// 
/// // Check if task can transition to InProgress
/// assert!(task.can_transition_to(TaskState::InProgress));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    /// Auto-increment primary key
    pub id: i32,
    /// Human-readable identifier (e.g., "ARCH-01", "DB-15")
    pub code: String,
    /// Brief task title
    pub name: String,
    /// Detailed task requirements
    pub description: String,
    /// Assigned agent identifier
    pub owner_agent_name: String,
    /// Current lifecycle state
    pub state: TaskState,
    /// Creation timestamp
    pub inserted_at: DateTime<Utc>,
    /// Completion timestamp
    pub done_at: Option<DateTime<Utc>>,
}

/// Task lifecycle states defining the progression of work.
/// 
/// Tasks move through a defined state machine with validated transitions.
/// The typical flow is: Created → InProgress → Review → Done → Archived,
/// with Blocked as a temporary state that can occur during InProgress.
/// 
/// # State Transitions
/// 
/// - `Created` → `InProgress`
/// - `InProgress` → `Blocked`, `Review`, `Done`  
/// - `Blocked` → `InProgress`
/// - `Review` → `InProgress`, `Done`
/// - `Done` → `Archived` (via archive_task only)
/// - `Archived` → (no transitions allowed)
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::models::{Task, TaskState};
/// use chrono::Utc;
/// 
/// let task = Task {
///     id: 1,
///     code: "TEST-01".to_string(),
///     name: "Test Task".to_string(),
///     description: "A test task".to_string(),
///     owner_agent_name: "test-agent".to_string(),
///     state: TaskState::Created,
///     inserted_at: Utc::now(),
///     done_at: None,
/// };
/// 
/// // Check valid transitions
/// if task.can_transition_to(TaskState::InProgress) {
///     // Safe to move to InProgress
/// }
/// ```
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState {
    /// Newly created task
    Created,
    /// Task is actively being worked on
    InProgress,
    /// Task is blocked and cannot proceed
    Blocked,
    /// Task is ready for review
    Review,
    /// Task has been completed
    Done,
    /// Task has been archived
    Archived,
}

/// Data transfer object for creating new tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    /// Human-readable identifier (e.g., "ARCH-01", "DB-15")
    pub code: String,
    /// Brief task title
    pub name: String,
    /// Detailed task requirements
    pub description: String,
    /// Assigned agent identifier
    pub owner_agent_name: String,
}

/// Data transfer object for updating existing tasks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTask {
    /// Optional new task name
    pub name: Option<String>,
    /// Optional new task description
    pub description: Option<String>,
    /// Optional new owner agent
    pub owner_agent_name: Option<String>,
}

/// Filter criteria for querying tasks.
///
/// All fields are optional to support flexible querying.
/// When multiple fields are specified, they are combined with AND logic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    /// Filter by owner agent name
    pub owner: Option<String>,
    
    /// Filter by task state
    pub state: Option<TaskState>,
    
    /// Filter tasks created on or after this date
    pub date_from: Option<DateTime<Utc>>,
    
    /// Filter tasks created on or before this date
    pub date_to: Option<DateTime<Utc>>,
}

impl Task {
    /// Check if the task can transition to the given state
    pub fn can_transition_to(&self, new_state: TaskState) -> bool {
        use TaskState::*;
        
        match (self.state, new_state) {
            // Can't transition to the same state
            (current, new) if current == new => false,
            
            // Valid transitions from Created
            (Created, InProgress) => true,
            
            // Valid transitions from InProgress
            (InProgress, Blocked | Review | Done) => true,
            
            // Valid transitions from Blocked
            (Blocked, InProgress) => true,
            
            // Valid transitions from Review
            (Review, InProgress | Done) => true,
            
            // Valid transitions from Done
            (Done, Archived) => true,
            
            // No valid transitions from Archived
            (Archived, _) => false,
            
            // All other transitions are invalid
            _ => false,
        }
    }
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::Created => write!(f, "Created"),
            TaskState::InProgress => write!(f, "InProgress"), 
            TaskState::Blocked => write!(f, "Blocked"),
            TaskState::Review => write!(f, "Review"),
            TaskState::Done => write!(f, "Done"),
            TaskState::Archived => write!(f, "Archived"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_state_transitions() {
        let task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        };

        // Created -> InProgress
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Archived));

        // InProgress -> Blocked, Review, Done
        let mut task = task;
        task.state = TaskState::InProgress;
        assert!(task.can_transition_to(TaskState::Blocked));
        assert!(task.can_transition_to(TaskState::Review));
        assert!(task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Blocked -> InProgress
        task.state = TaskState::Blocked;
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Review -> InProgress, Done
        task.state = TaskState::Review;
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Done -> Archived
        task.state = TaskState::Done;
        assert!(task.can_transition_to(TaskState::Archived));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));

        // Archived -> nothing
        task.state = TaskState::Archived;
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
    }

    #[test]
    fn test_no_same_state_transition() {
        let task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::InProgress,
            inserted_at: Utc::now(),
            done_at: None,
        };

        // Cannot transition to the same state
        assert!(!task.can_transition_to(TaskState::InProgress));
    }
}--- END DEFINITION ---
[INTERFACE-ERROR-TYPES] 2025-07-29 15:41:39 rust-architect: ERROR-TYPES trait ready
--- BEGIN DEFINITION ---
use thiserror::Error;
use crate::models::TaskState;

/// Result type alias for task operations
pub type Result<T> = std::result::Result<T, TaskError>;

/// Comprehensive error types for the MCP Task Management System.
/// 
/// These errors cover all possible failure modes in task operations,
/// from validation failures to database errors. Each error type maps
/// to appropriate HTTP status codes for API responses.
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::error::{TaskError, Result};
/// use task_core::models::TaskState;
/// 
/// // Create specific error types
/// let not_found = TaskError::not_found_id(42);
/// let invalid_transition = TaskError::invalid_transition(
///     TaskState::Created, 
///     TaskState::Done
/// );
/// 
/// // Check error categories
/// assert!(not_found.is_not_found());
/// assert_eq!(not_found.status_code(), 404);
/// assert_eq!(invalid_transition.status_code(), 422);
/// ```
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    /// Task not found by the given identifier
    #[error("Task not found: {0}")]
    NotFound(String),

    /// Invalid state transition attempted
    #[error("Invalid state transition from {0} to {1}")]
    InvalidStateTransition(TaskState, TaskState),

    /// Duplicate task code already exists
    #[error("Task code already exists: {0}")]
    DuplicateCode(String),

    /// Validation error with details
    #[error("Validation error: {0}")]
    Validation(String),

    /// Database operation error
    #[error("Database error: {0}")]
    Database(String),

    /// Protocol error from MCP operations
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Internal system error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl TaskError {
    /// Create a not found error for a task ID
    pub fn not_found_id(id: i32) -> Self {
        Self::NotFound(format!("Task with ID {id} not found"))
    }

    /// Create a not found error for a task code
    pub fn not_found_code(code: &str) -> Self {
        Self::NotFound(format!("Task with code '{code}' not found"))
    }

    /// Create a validation error for invalid task code format
    pub fn invalid_code_format(code: &str) -> Self {
        Self::Validation(format!("Invalid task code format: '{code}'"))
    }

    /// Create a validation error for invalid agent name
    pub fn invalid_agent_name(name: &str) -> Self {
        Self::Validation(format!("Invalid agent name: '{name}'"))
    }

    /// Create a validation error for empty field
    pub fn empty_field(field: &str) -> Self {
        Self::Validation(format!("Field '{field}' cannot be empty"))
    }

    /// Create a state transition error
    pub fn invalid_transition(from: TaskState, to: TaskState) -> Self {
        Self::InvalidStateTransition(from, to)
    }

    /// Check if this error indicates a not found condition
    pub fn is_not_found(&self) -> bool {
        matches!(self, TaskError::NotFound(_))
    }

    /// Check if this error indicates a validation problem
    pub fn is_validation(&self) -> bool {
        matches!(self, TaskError::Validation(_))
    }

    /// Check if this error indicates a database problem
    pub fn is_database(&self) -> bool {
        matches!(self, TaskError::Database(_))
    }

    /// Convert to appropriate HTTP status code equivalent
    pub fn status_code(&self) -> u16 {
        match self {
            TaskError::NotFound(_) => 404,
            TaskError::Validation(_) => 400,
            TaskError::DuplicateCode(_) => 409,
            TaskError::InvalidStateTransition(_, _) => 422,
            TaskError::Database(_) => 500,
            TaskError::Protocol(_) => 500,
            TaskError::Configuration(_) => 500,
            TaskError::Internal(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = TaskError::not_found_id(42);
        assert_eq!(error, TaskError::NotFound("Task with ID 42 not found".to_string()));
        assert!(error.is_not_found());
        assert_eq!(error.status_code(), 404);

        let error = TaskError::not_found_code("ARCH-01");
        assert_eq!(error, TaskError::NotFound("Task with code 'ARCH-01' not found".to_string()));
        
        let error = TaskError::invalid_code_format("invalid-code");
        assert!(error.is_validation());
        assert_eq!(error.status_code(), 400);

        let error = TaskError::invalid_transition(TaskState::Created, TaskState::Done);
        assert_eq!(error, TaskError::InvalidStateTransition(TaskState::Created, TaskState::Done));
        assert_eq!(error.status_code(), 422);
    }

    #[test]
    fn test_error_display() {
        let error = TaskError::NotFound("Task not found".to_string());
        assert_eq!(format!("{}", error), "Task not found: Task not found");

        let error = TaskError::InvalidStateTransition(TaskState::Created, TaskState::Done);
        assert_eq!(format!("{}", error), "Invalid state transition from Created to Done");

        let error = TaskError::Validation("Invalid input".to_string());
        assert_eq!(format!("{}", error), "Validation error: Invalid input");
    }

    #[test]
    fn test_error_predicates() {
        assert!(TaskError::NotFound("test".to_string()).is_not_found());
        assert!(!TaskError::Validation("test".to_string()).is_not_found());

        assert!(TaskError::Validation("test".to_string()).is_validation());
        assert!(!TaskError::Database("test".to_string()).is_validation());

        assert!(TaskError::Database("test".to_string()).is_database());
        assert!(!TaskError::Protocol("test".to_string()).is_database());
    }
}--- END DEFINITION ---
[INTERFACE-MOCK-REPOSITORY] 2025-07-29 16:46:28 testing-expert: MOCK-REPOSITORY trait ready
--- BEGIN DEFINITION ---
//! Mock implementation of TaskRepository trait
//! 
//! Provides a thread-safe mock repository with:
//! - Error injection capabilities
//! - Call tracking for verification
//! - Realistic behavior simulation

use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicI32, Ordering}};
use parking_lot::Mutex;
use async_trait::async_trait;
use chrono::Utc;
use task_core::{
    Task, TaskState, TaskRepository, TaskError, Result, NewTask, UpdateTask, TaskFilter,
    repository::RepositoryStats
};

/// Mock implementation of TaskRepository for testing
/// 
/// Features:
/// - Thread-safe concurrent access
/// - Error injection for failure testing
/// - Call history tracking for verification
/// - Realistic behavior simulation
pub struct MockTaskRepository {
    tasks: Arc<Mutex<HashMap<i32, Task>>>,
    next_id: Arc<AtomicI32>,
    error_injection: Arc<Mutex<Option<TaskError>>>,
    call_history: Arc<Mutex<Vec<String>>>,
}

impl Default for MockTaskRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTaskRepository {
    /// Create a new empty mock repository
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicI32::new(1)),
            error_injection: Arc::new(Mutex::new(None)),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create mock repository with pre-populated tasks
    pub fn with_tasks(tasks: Vec<Task>) -> Self {
        let mut task_map = HashMap::new();
        let mut max_id = 0;
        
        for task in tasks {
            if task.id > max_id {
                max_id = task.id;
            }
            task_map.insert(task.id, task);
        }
        
        Self {
            tasks: Arc::new(Mutex::new(task_map)),
            next_id: Arc::new(AtomicI32::new(max_id + 1)),
            error_injection: Arc::new(Mutex::new(None)),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create mock repository with specific starting ID
    pub fn with_next_id(next_id: i32) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicI32::new(next_id)),
            error_injection: Arc::new(Mutex::new(None)),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Inject error for next operation
    pub fn inject_error(&self, error: TaskError) {
        *self.error_injection.lock() = Some(error);
    }

    /// Clear error injection
    pub fn clear_error(&self) {
        *self.error_injection.lock() = None;
    }

    /// Get history of called methods
    pub fn call_history(&self) -> Vec<String> {
        self.call_history.lock().clone()
    }

    /// Clear call history
    pub fn clear_history(&self) {
        self.call_history.lock().clear();
    }

    /// Assert method was called
    pub fn assert_called(&self, method: &str) {
        let history = self.call_history.lock();
        assert!(
            history.iter().any(|call| call.contains(method)),
            "Method '{}' was not called. Call history: {:?}",
            method,
            *history
        );
    }

    /// Check if an error should be injected, consuming it if so
    fn check_error_injection(&self) -> Result<()> {
        let mut error_opt = self.error_injection.lock();
        if let Some(error) = error_opt.take() {
            return Err(error);
        }
        Ok(())
    }

    /// Record method call in history
    fn record_call(&self, method: &str) {
        self.call_history.lock().push(format!("{method}()"));
    }

    /// Record method call with parameters in history
    fn record_call_with_params(&self, method: &str, params: &str) {
        self.call_history.lock().push(format!("{method}({params})"));
    }
}

#[async_trait]
impl TaskRepository for MockTaskRepository {
    async fn create(&self, task: NewTask) -> Result<Task> {
        self.record_call_with_params("create", &format!("code={}", task.code));
        
        // Check for error injection
        self.check_error_injection()?;
        
        // Check for duplicate code
        let tasks = self.tasks.lock();
        if tasks.values().any(|t| t.code == task.code) {
            return Err(TaskError::DuplicateCode(task.code));
        }
        drop(tasks);
        
        // Create task with next ID
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let now = Utc::now();
        
        let new_task = Task {
            id,
            code: task.code,
            name: task.name,
            description: task.description,
            owner_agent_name: task.owner_agent_name,
            state: TaskState::Created,
            inserted_at: now,
            done_at: None,
        };
        
        // Store in HashMap
        self.tasks.lock().insert(id, new_task.clone());
        
        Ok(new_task)
    }
    
    async fn update(&self, id: i32, updates: UpdateTask) -> Result<Task> {
        self.record_call_with_params("update", &format!("id={id}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&id).ok_or_else(|| TaskError::NotFound(id.to_string()))?;
        
        // Apply updates
        if let Some(name) = updates.name {
            task.name = name;
        }
        if let Some(description) = updates.description {
            task.description = description;
        }
        if let Some(owner) = updates.owner_agent_name {
            task.owner_agent_name = owner;
        }
        
        Ok(task.clone())
    }
    
    async fn set_state(&self, id: i32, state: TaskState) -> Result<Task> {
        self.record_call_with_params("set_state", &format!("id={id}, state={state}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&id).ok_or_else(|| TaskError::NotFound(id.to_string()))?;
        
        // Validate state transition
        if !task.can_transition_to(state) {
            return Err(TaskError::InvalidStateTransition(task.state, state));
        }
        
        task.state = state;
        
        // Set completion timestamp if moving to Done
        if state == TaskState::Done {
            task.done_at = Some(Utc::now());
        }
        
        Ok(task.clone())
    }
    
    async fn get_by_id(&self, id: i32) -> Result<Option<Task>> {
        self.record_call_with_params("get_by_id", &format!("id={id}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        Ok(tasks.get(&id).cloned())
    }
    
    async fn get_by_code(&self, code: &str) -> Result<Option<Task>> {
        self.record_call_with_params("get_by_code", &format!("code={code}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        Ok(tasks.values().find(|t| t.code == code).cloned())
    }
    
    async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>> {
        self.record_call("list");
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        let mut result: Vec<Task> = tasks.values()
            .filter(|task| {
                // Filter by owner
                if let Some(ref owner) = filter.owner {
                    if task.owner_agent_name != *owner {
                        return false;
                    }
                }
                
                // Filter by state
                if let Some(state) = filter.state {
                    if task.state != state {
                        return false;
                    }
                }
                
                // Filter by date range
                if let Some(date_from) = filter.date_from {
                    if task.inserted_at < date_from {
                        return false;
                    }
                }
                
                if let Some(date_to) = filter.date_to {
                    if task.inserted_at > date_to {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // Sort by creation date (most recent first)
        result.sort_by(|a, b| b.inserted_at.cmp(&a.inserted_at));
        
        Ok(result)
    }
    
    async fn assign(&self, id: i32, new_owner: &str) -> Result<Task> {
        self.record_call_with_params("assign", &format!("id={id}, owner={new_owner}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        // Validate owner name is not empty
        if new_owner.trim().is_empty() {
            return Err(TaskError::Validation("Owner name cannot be empty".to_string()));
        }
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&id).ok_or_else(|| TaskError::NotFound(id.to_string()))?;
        
        task.owner_agent_name = new_owner.to_string();
        
        Ok(task.clone())
    }
    
    async fn archive(&self, id: i32) -> Result<Task> {
        self.record_call_with_params("archive", &format!("id={id}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&id).ok_or_else(|| TaskError::NotFound(id.to_string()))?;
        
        // Validate that task can be archived
        if !task.can_transition_to(TaskState::Archived) {
            return Err(TaskError::InvalidStateTransition(task.state, TaskState::Archived));
        }
        
        task.state = TaskState::Archived;
        
        Ok(task.clone())
    }
    
    async fn health_check(&self) -> Result<()> {
        self.record_call("health_check");
        
        // Check for error injection
        self.check_error_injection()?;
        
        // Mock always reports healthy
        Ok(())
    }
    
    async fn get_stats(&self) -> Result<RepositoryStats> {
        self.record_call("get_stats");
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        let mut stats = RepositoryStats {
            total_tasks: tasks.len() as u64,
            ..Default::default()
        };
        
        // Count tasks by state
        for task in tasks.values() {
            *stats.tasks_by_state.entry(task.state).or_insert(0) += 1;
        }
        
        // Count tasks by owner
        for task in tasks.values() {
            *stats.tasks_by_owner.entry(task.owner_agent_name.clone()).or_insert(0) += 1;
        }
        
        // Find latest timestamps
        stats.latest_created = tasks.values()
            .map(|t| t.inserted_at)
            .max();
        
        stats.latest_completed = tasks.values()
            .filter_map(|t| t.done_at)
            .max();
        
        Ok(stats)
    }
}--- END DEFINITION ---
[INTERFACE-CONTRACT-TESTS] 2025-07-29 16:46:45 testing-expert: CONTRACT-TESTS trait ready
--- BEGIN DEFINITION ---
//! Contract test helpers for validating trait implementations
//! 
//! Provides standardized tests that any implementation of core traits
//! should pass, ensuring consistent behavior across different implementations.

use task_core::{TaskRepository, TaskState, TaskError};
use crate::{create_new_task, NewTaskBuilder, UpdateTaskBuilder, TaskFilterBuilder};

/// Test any TaskRepository implementation with comprehensive contract tests
/// 
/// This function runs a suite of tests that any TaskRepository implementation
/// should pass to be considered compliant with the expected contract.
pub async fn test_repository_contract<R: TaskRepository>(repo: &R) {
    test_create_contract(repo).await;
    test_update_contract(repo).await;
    test_state_contract(repo).await;
    test_get_contract(repo).await;
    test_list_contract(repo).await;
    test_assign_contract(repo).await;
    test_archive_contract(repo).await;
    test_health_check_contract(repo).await;
    test_stats_contract(repo).await;
}

/// Test task creation contract
pub async fn test_create_contract<R: TaskRepository>(repo: &R) {
    // Test successful creation
    let new_task = create_new_task();
    let task = repo.create(new_task.clone()).await.expect("Create should succeed");
    
    assert!(task.id > 0, "Created task should have positive ID");
    assert_eq!(task.code, new_task.code, "Created task should preserve code");
    assert_eq!(task.name, new_task.name, "Created task should preserve name");
    assert_eq!(task.state, TaskState::Created, "New task should start in Created state");
    assert!(task.done_at.is_none(), "New task should not have done_at timestamp");
    
    // Test duplicate code rejection
    let duplicate_result = repo.create(new_task).await;
    assert!(duplicate_result.is_err(), "Should reject duplicate task codes");
    match duplicate_result.unwrap_err() {
        TaskError::DuplicateCode(_) => {}, // Expected
        other => panic!("Expected DuplicateCode error, got: {other:?}"),
    }
}

/// Test task update contract
pub async fn test_update_contract<R: TaskRepository>(repo: &R) {
    // Create a task first
    let new_task = NewTaskBuilder::new()
        .with_code("UPDATE-TEST")
        .build();
    let task = repo.create(new_task).await.expect("Create should succeed");
    
    // Test successful update
    let update = UpdateTaskBuilder::new()
        .with_name("Updated Name")
        .with_description("Updated Description")
        .build();
    
    let updated_task = repo.update(task.id, update).await.expect("Update should succeed");
    assert_eq!(updated_task.name, "Updated Name");
    assert_eq!(updated_task.description, "Updated Description");
    assert_eq!(updated_task.id, task.id, "ID should remain unchanged");
    assert_eq!(updated_task.code, task.code, "Code should remain unchanged");
    
    // Test update non-existent task
    let update_result = repo.update(99999, UpdateTaskBuilder::new().build()).await;
    assert!(update_result.is_err(), "Should fail to update non-existent task");
    match update_result.unwrap_err() {
        TaskError::NotFound(_) => {}, // Expected
        other => panic!("Expected NotFound error, got: {other:?}"),
    }
}

/// Test state transition contract
pub async fn test_state_contract<R: TaskRepository>(repo: &R) {
    // Create a task first
    let new_task = NewTaskBuilder::new()
        .with_code("STATE-TEST")
        .build();
    let task = repo.create(new_task).await.expect("Create should succeed");
    
    // Test valid state transition
    let updated_task = repo.set_state(task.id, TaskState::InProgress).await
        .expect("Valid state transition should succeed");
    assert_eq!(updated_task.state, TaskState::InProgress);
    
    // Test completion sets done_at
    let done_task = repo.set_state(task.id, TaskState::Done).await
        .expect("Transition to Done should succeed");
    assert_eq!(done_task.state, TaskState::Done);
    assert!(done_task.done_at.is_some(), "Done task should have done_at timestamp");
    
    // Test invalid state transition (Done -> InProgress is not allowed)
    let invalid_result = repo.set_state(task.id, TaskState::InProgress).await;
    assert!(invalid_result.is_err(), "Should reject invalid state transition");
    match invalid_result.unwrap_err() {
        TaskError::InvalidStateTransition(_, _) => {}, // Expected
        other => panic!("Expected InvalidStateTransition error, got: {other:?}"),
    }
    
    // Test state change on non-existent task
    let not_found_result = repo.set_state(99999, TaskState::InProgress).await;
    assert!(not_found_result.is_err(), "Should fail for non-existent task");
}

/// Test get operations contract
pub async fn test_get_contract<R: TaskRepository>(repo: &R) {
    // Create a task first
    let new_task = NewTaskBuilder::new()
        .with_code("GET-TEST")
        .build();
    let task = repo.create(new_task).await.expect("Create should succeed");
    
    // Test get by ID
    let retrieved_by_id = repo.get_by_id(task.id).await
        .expect("Get by ID should succeed")
        .expect("Task should exist");
    assert_eq!(retrieved_by_id.id, task.id);
    assert_eq!(retrieved_by_id.code, task.code);
    
    // Test get by code
    let retrieved_by_code = repo.get_by_code(&task.code).await
        .expect("Get by code should succeed")
        .expect("Task should exist");
    assert_eq!(retrieved_by_code.id, task.id);
    assert_eq!(retrieved_by_code.code, task.code);
    
    // Test get non-existent by ID
    let not_found_by_id = repo.get_by_id(99999).await
        .expect("Get by ID should not error for non-existent ID");
    assert!(not_found_by_id.is_none(), "Should return None for non-existent ID");
    
    // Test get non-existent by code
    let not_found_by_code = repo.get_by_code("NON-EXISTENT").await
        .expect("Get by code should not error for non-existent code");
    assert!(not_found_by_code.is_none(), "Should return None for non-existent code");
}

/// Test list operations contract
pub async fn test_list_contract<R: TaskRepository>(repo: &R) {
    // Create multiple tasks with different properties
    let tasks = vec![
        NewTaskBuilder::new().with_code("LIST-1").with_owner_agent_name("agent-1").build(),
        NewTaskBuilder::new().with_code("LIST-2").with_owner_agent_name("agent-2").build(),
        NewTaskBuilder::new().with_code("LIST-3").with_owner_agent_name("agent-1").build(),
    ];
    
    let mut created_tasks = Vec::new();
    for new_task in tasks {
        let task = repo.create(new_task).await.expect("Create should succeed");
        created_tasks.push(task);
    }
    
    // Set different states
    repo.set_state(created_tasks[1].id, TaskState::InProgress).await
        .expect("State change should succeed");
    
    // Test list all
    let all_tasks = repo.list(TaskFilterBuilder::new().build()).await
        .expect("List all should succeed");
    assert!(all_tasks.len() >= 3, "Should contain at least our created tasks");
    
    // Test filter by owner
    let agent1_tasks = repo.list(TaskFilterBuilder::new().with_owner("agent-1").build()).await
        .expect("Filter by owner should succeed");
    assert!(
        agent1_tasks.iter().all(|t| t.owner_agent_name == "agent-1"),
        "All returned tasks should be owned by agent-1"
    );
    
    // Test filter by state
    let in_progress_tasks = repo.list(TaskFilterBuilder::new().with_state(TaskState::InProgress).build()).await
        .expect("Filter by state should succeed");
    assert!(
        in_progress_tasks.iter().all(|t| t.state == TaskState::InProgress),
        "All returned tasks should be in InProgress state"
    );
}

/// Test task assignment contract
pub async fn test_assign_contract<R: TaskRepository>(repo: &R) {
    // Create a task first
    let new_task = NewTaskBuilder::new()
        .with_code("ASSIGN-TEST")
        .with_owner_agent_name("original-owner")
        .build();
    let task = repo.create(new_task).await.expect("Create should succeed");
    
    // Test successful assignment
    let assigned_task = repo.assign(task.id, "new-owner").await
        .expect("Assignment should succeed");
    assert_eq!(assigned_task.owner_agent_name, "new-owner");
    assert_eq!(assigned_task.id, task.id, "ID should remain unchanged");
    
    // Test assignment to empty owner (should fail)
    let empty_owner_result = repo.assign(task.id, "").await;
    assert!(empty_owner_result.is_err(), "Should reject empty owner name");
    
    // Test assignment of non-existent task
    let not_found_result = repo.assign(99999, "some-owner").await;
    assert!(not_found_result.is_err(), "Should fail for non-existent task");
}

/// Test task archival contract
pub async fn test_archive_contract<R: TaskRepository>(repo: &R) {
    // Create and complete a task first
    let new_task = NewTaskBuilder::new()
        .with_code("ARCHIVE-TEST")
        .build();
    let task = repo.create(new_task).await.expect("Create should succeed");
    
    // Follow valid state transitions: Created -> InProgress -> Done
    let in_progress_task = repo.set_state(task.id, TaskState::InProgress).await
        .expect("Set to InProgress should succeed");
    let done_task = repo.set_state(in_progress_task.id, TaskState::Done).await
        .expect("Set to Done should succeed");
    
    // Test successful archival
    let archived_task = repo.archive(done_task.id).await
        .expect("Archive should succeed");
    assert_eq!(archived_task.state, TaskState::Archived);
    assert_eq!(archived_task.id, task.id, "ID should remain unchanged");
    
    // Test archival of task in invalid state
    let new_task2 = NewTaskBuilder::new()
        .with_code("ARCHIVE-TEST-2")
        .build();
    let task2 = repo.create(new_task2).await.expect("Create should succeed");
    
    let invalid_archive_result = repo.archive(task2.id).await;
    assert!(invalid_archive_result.is_err(), "Should reject archival of non-completed task");
    
    // Test archival of non-existent task
    let not_found_result = repo.archive(99999).await;
    assert!(not_found_result.is_err(), "Should fail for non-existent task");
}

/// Test health check contract
pub async fn test_health_check_contract<R: TaskRepository>(repo: &R) {
    // Health check should succeed for a working repository
    let health_result = repo.health_check().await;
    assert!(health_result.is_ok(), "Health check should succeed for working repository");
}

/// Test statistics contract
pub async fn test_stats_contract<R: TaskRepository>(repo: &R) {
    // Create some tasks for statistics
    let new_task = NewTaskBuilder::new()
        .with_code("STATS-TEST")
        .build();
    let _task = repo.create(new_task).await.expect("Create should succeed");
    
    // Get stats
    let stats = repo.get_stats().await.expect("Get stats should succeed");
    
    // Verify basic stats structure
    assert!(stats.total_tasks > 0, "Should report at least one task");
    assert!(!stats.tasks_by_state.is_empty(), "Should have state breakdown");
    assert!(!stats.tasks_by_owner.is_empty(), "Should have owner breakdown");
    assert!(stats.latest_created.is_some(), "Should have latest creation timestamp");
}--- END DEFINITION ---
[INTERFACE-TASK-FILTER] 2025-07-29 20:45:16 rust-architect: TASK-FILTER trait ready
--- BEGIN DEFINITION ---
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Core task representation in the MCP Task Management System.
/// 
/// A task represents a unit of work that can be tracked through its lifecycle,
/// assigned to agents, and managed via the MCP protocol. Each task has a unique
/// numeric ID and human-readable code for easy reference.
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::models::{Task, TaskState};
/// use chrono::Utc;
/// 
/// let task = Task {
///     id: 42,
///     code: "FEAT-001".to_string(),
///     name: "Implement user authentication".to_string(),
///     description: "Add JWT-based auth with role-based access control".to_string(),
///     owner_agent_name: "backend-developer".to_string(),
///     state: TaskState::Created,
///     inserted_at: Utc::now(),
///     done_at: None,
/// };
/// 
/// // Check if task can transition to InProgress
/// assert!(task.can_transition_to(TaskState::InProgress));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    /// Auto-increment primary key
    pub id: i32,
    /// Human-readable identifier (e.g., "ARCH-01", "DB-15")
    pub code: String,
    /// Brief task title
    pub name: String,
    /// Detailed task requirements
    pub description: String,
    /// Assigned agent identifier
    pub owner_agent_name: String,
    /// Current lifecycle state
    pub state: TaskState,
    /// Creation timestamp
    pub inserted_at: DateTime<Utc>,
    /// Completion timestamp
    pub done_at: Option<DateTime<Utc>>,
}

/// Task lifecycle states defining the progression of work.
/// 
/// Tasks move through a defined state machine with validated transitions.
/// The typical flow is: Created → InProgress → Review → Done → Archived,
/// with Blocked as a temporary state that can occur during InProgress.
/// 
/// # State Transitions
/// 
/// - `Created` → `InProgress`
/// - `InProgress` → `Blocked`, `Review`, `Done`  
/// - `Blocked` → `InProgress`
/// - `Review` → `InProgress`, `Done`
/// - `Done` → `Archived` (via archive_task only)
/// - `Archived` → (no transitions allowed)
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::models::{Task, TaskState};
/// use chrono::Utc;
/// 
/// let task = Task {
///     id: 1,
///     code: "TEST-01".to_string(),
///     name: "Test Task".to_string(),
///     description: "A test task".to_string(),
///     owner_agent_name: "test-agent".to_string(),
///     state: TaskState::Created,
///     inserted_at: Utc::now(),
///     done_at: None,
/// };
/// 
/// // Check valid transitions
/// if task.can_transition_to(TaskState::InProgress) {
///     // Safe to move to InProgress
/// }
/// ```
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState {
    /// Newly created task
    Created,
    /// Task is actively being worked on
    InProgress,
    /// Task is blocked and cannot proceed
    Blocked,
    /// Task is ready for review
    Review,
    /// Task has been completed
    Done,
    /// Task has been archived
    Archived,
}

/// Data transfer object for creating new tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewTask {
    /// Human-readable identifier (e.g., "ARCH-01", "DB-15")
    pub code: String,
    /// Brief task title
    pub name: String,
    /// Detailed task requirements
    pub description: String,
    /// Assigned agent identifier
    pub owner_agent_name: String,
}

/// Data transfer object for updating existing tasks
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct UpdateTask {
    /// Optional new task name
    pub name: Option<String>,
    /// Optional new task description
    pub description: Option<String>,
    /// Optional new owner agent
    pub owner_agent_name: Option<String>,
}

/// Filter criteria for querying tasks.
///
/// All fields are optional to support flexible querying.
/// When multiple fields are specified, they are combined with AND logic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    /// Filter by owner agent name
    pub owner: Option<String>,
    
    /// Filter by task state
    pub state: Option<TaskState>,
    
    /// Filter tasks created on or after this date
    pub date_from: Option<DateTime<Utc>>,
    
    /// Filter tasks created on or before this date
    pub date_to: Option<DateTime<Utc>>,
    
    /// Filter tasks completed on or after this date
    pub completed_after: Option<DateTime<Utc>>,
    
    /// Filter tasks completed on or before this date
    pub completed_before: Option<DateTime<Utc>>,
    
    /// Maximum number of tasks to return (for pagination)
    pub limit: Option<u32>,
    
    /// Number of tasks to skip (for pagination)
    pub offset: Option<u32>,
}

impl Task {
    /// Check if the task can transition to the given state
    pub fn can_transition_to(&self, new_state: TaskState) -> bool {
        use TaskState::*;
        
        match (self.state, new_state) {
            // Can't transition to the same state
            (current, new) if current == new => false,
            
            // Valid transitions from Created
            (Created, InProgress) => true,
            
            // Valid transitions from InProgress
            (InProgress, Blocked | Review | Done) => true,
            
            // Valid transitions from Blocked
            (Blocked, InProgress) => true,
            
            // Valid transitions from Review
            (Review, InProgress | Done) => true,
            
            // Valid transitions from Done
            (Done, Archived) => true,
            
            // No valid transitions from Archived
            (Archived, _) => false,
            
            // All other transitions are invalid
            _ => false,
        }
    }
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::Created => write!(f, "Created"),
            TaskState::InProgress => write!(f, "InProgress"), 
            TaskState::Blocked => write!(f, "Blocked"),
            TaskState::Review => write!(f, "Review"),
            TaskState::Done => write!(f, "Done"),
            TaskState::Archived => write!(f, "Archived"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_state_transitions() {
        let task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        };

        // Created -> InProgress
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Archived));

        // InProgress -> Blocked, Review, Done
        let mut task = task;
        task.state = TaskState::InProgress;
        assert!(task.can_transition_to(TaskState::Blocked));
        assert!(task.can_transition_to(TaskState::Review));
        assert!(task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Blocked -> InProgress
        task.state = TaskState::Blocked;
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Review -> InProgress, Done
        task.state = TaskState::Review;
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Done -> Archived
        task.state = TaskState::Done;
        assert!(task.can_transition_to(TaskState::Archived));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));

        // Archived -> nothing
        task.state = TaskState::Archived;
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
    }

    #[test]
    fn test_no_same_state_transition() {
        let task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::InProgress,
            inserted_at: Utc::now(),
            done_at: None,
        };

        // Cannot transition to the same state
        assert!(!task.can_transition_to(TaskState::InProgress));
    }
}--- END DEFINITION ---
[INTERFACE-UPDATE-TASK-PARAMS] 2025-07-29 20:45:39 rust-architect: UPDATE-TASK-PARAMS trait ready
--- BEGIN DEFINITION ---
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{
    error::Result,
    models::{Task, TaskFilter, TaskState, NewTask, UpdateTask},
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
/// 
/// This is a wrapper around the core NewTask model that provides MCP-specific
/// serialization and validation while reusing the domain model.
pub type CreateTaskParams = NewTask;

/// MCP parameters for updating a task
/// 
/// Contains the task ID and the update data. The update data reuses
/// the core UpdateTask model to avoid duplication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskParams {
    pub id: i32,
    #[serde(flatten)]
    pub update_data: UpdateTask,
}

impl UpdateTaskParams {
    /// Extract the update data for use with repository layer
    pub fn into_update_data(self) -> UpdateTask {
        self.update_data
    }

    /// Get a reference to the update data
    pub fn update_data(&self) -> &UpdateTask {
        &self.update_data
    }

    /// Backward compatibility accessors for individual fields
    pub fn name(&self) -> &Option<String> {
        &self.update_data.name
    }

    pub fn description(&self) -> &Option<String> {
        &self.update_data.description
    }

    pub fn owner_agent_name(&self) -> &Option<String> {
        &self.update_data.owner_agent_name
    }
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

        let completed_after = match &self.completed_after {
            Some(s) => Some(parse_datetime(s)?),
            None => None,
        };

        let completed_before = match &self.completed_before {
            Some(s) => Some(parse_datetime(s)?),
            None => None,
        };

        Ok(TaskFilter {
            owner: self.owner_agent_name.clone(),
            state: self.state,
            date_from: created_after,
            date_to: created_before,
            completed_after,
            completed_before,
            limit: self.limit,
            offset: None, // Currently not exposed in MCP protocol, but could be added later
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
            completed_after: Some("2023-12-15T00:00:00Z".to_string()),
            completed_before: Some("2023-12-30T23:59:59Z".to_string()),
            limit: Some(10),
        };

        let filter = params.to_task_filter().unwrap();
        assert_eq!(filter.owner, Some("test-agent".to_string()));
        assert_eq!(filter.state, Some(TaskState::InProgress));
        assert!(filter.date_from.is_some());
        assert!(filter.date_to.is_some());
        assert!(filter.completed_after.is_some());
        assert!(filter.completed_before.is_some());
        assert_eq!(filter.limit, Some(10));
        assert_eq!(filter.offset, None);
    }

    #[test]
    fn test_update_task_params_methods() {
        let update_data = UpdateTask {
            name: Some("Updated Task".to_string()),
            description: Some("Updated description".to_string()),
            owner_agent_name: Some("new-owner".to_string()),
        };

        let params = UpdateTaskParams {
            id: 42,
            update_data: update_data.clone(),
        };

        assert_eq!(params.id, 42);
        assert_eq!(params.name(), &Some("Updated Task".to_string()));
        assert_eq!(params.description(), &Some("Updated description".to_string()));
        assert_eq!(params.owner_agent_name(), &Some("new-owner".to_string()));
        assert_eq!(params.update_data(), &update_data);

        let extracted = params.into_update_data();
        assert_eq!(extracted.name, Some("Updated Task".to_string()));
        assert_eq!(extracted.description, Some("Updated description".to_string()));
        assert_eq!(extracted.owner_agent_name, Some("new-owner".to_string()));
    }

    #[test]
    fn test_health_status_default() {
        let health = HealthStatus::default();
        assert_eq!(health.status, "unknown");
        assert!(!health.database);
        assert!(!health.protocol);
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    }
}--- END DEFINITION ---
