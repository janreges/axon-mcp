//! Task Core Library
//!
//! This crate provides the foundational domain models, business logic, and trait interfaces
//! for the task management system. All other crates depend on the types and interfaces
//! defined here.
//!
//! # Architecture
//!
//! The crate is organized into the following modules:
//!
//! - [`models`] - Core domain models (Task, TaskState, etc.)
//! - [`error`] - Error types and result handling
//! - [`repository`] - Repository trait for data persistence
//! - [`protocol`] - Protocol handler trait for MCP operations
//! - [`validation`] - Business logic validation utilities
//!
//! # Example
//!
//! ```rust
//! use task_core::{
//!     models::{NewTask, TaskState},
//!     validation::TaskValidator,
//! };
//!
//! let new_task = NewTask::new(
//!     "ARCH-01".to_string(),
//!     "System Architecture".to_string(),
//!     "Design the overall system architecture".to_string(),
//!     Some("rust-architect".to_string()),
//! );
//!
//! // Validate the task before creation
//! TaskValidator::validate_new_task(&new_task).unwrap();
//! ```

pub mod models;
pub mod error;
pub mod repository;
pub mod protocol;
pub mod validation;
pub mod circuit_breaker;
pub mod mcp_v2_extensions;

// Re-export commonly used types at the crate root for convenience
pub use models::{
    Task, TaskState, TaskFilter, NewTask, UpdateTask,
    // MCP v2 New Entity Types
    KnowledgeObject, TaskMessage, AgentProfile, AgentStatus,
    WorkflowDefinition, SystemEvent, EventSeverity, WorkSession,
};
pub use error::{TaskError, Result};
pub use repository::{TaskRepository, TaskMessageRepository, RepositoryStats};
pub use protocol::{
    ProtocolHandler, HealthStatus,
    CreateTaskParams, UpdateTaskParams, SetStateParams,
    GetTaskByIdParams, GetTaskByCodeParams, ListTasksParams,
    AssignTaskParams, ArchiveTaskParams,
    // MCP v2 Advanced Multi-Agent Types
    DiscoverWorkParams, ClaimTaskParams, ReleaseTaskParams,
    StartWorkSessionParams, EndWorkSessionParams, WorkSessionInfo,
    // Task Messaging Types
    CreateTaskMessageParams, GetTaskMessagesParams,
};
pub use validation::TaskValidator;
pub use circuit_breaker::{CircuitBreaker, FailureType, CircuitState, CircuitBreakerAction};
pub use mcp_v2_extensions::{
    DiscoverWorkResponse, PrerequisiteAction, WorkDiscoveryConfig, ClaimResult,
    CapabilityMatcher, PriorityCalculator, SimpleWorkSession, AgentWorkload, SimpleKnowledgeEntry,
};

/// Current version of the core crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current crate name
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_constants() {
        assert!(!VERSION.is_empty());
        assert_eq!(CRATE_NAME, "task-core");
    }

    #[test]
    fn test_re_exports() {
        use crate::{TaskState, TaskError};
        
        // Test that re-exports work
        let state = TaskState::Created;
        assert_eq!(format!("{}", state), "Created");
        
        let error = TaskError::not_found_id(1);
        assert!(error.is_not_found());
    }
}