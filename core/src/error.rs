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

    // MCP v2 Multi-Agent Errors

    /// Task is already claimed by another agent
    #[error("Task {0} is already claimed by {1}")]
    AlreadyClaimed(i32, String),

    /// Agent does not own the specified task
    #[error("Agent {0} does not own task {1}")]
    NotOwned(String, i32),

    /// Agent lacks required capabilities for task
    #[error("Agent {0} lacks required capabilities: {1:?}")]
    InsufficientCapabilities(String, Vec<String>),

    /// Work session not found or already ended
    #[error("Work session {0} not found or already ended")]
    SessionNotFound(i32),
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
            // MCP v2 Multi-Agent Errors
            TaskError::AlreadyClaimed(_, _) => 409, // Conflict
            TaskError::NotOwned(_, _) => 403, // Forbidden
            TaskError::InsufficientCapabilities(_, _) => 422, // Unprocessable Entity
            TaskError::SessionNotFound(_) => 404, // Not Found
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
}