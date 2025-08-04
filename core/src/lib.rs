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

pub mod ai_tool_adapters;
pub mod circuit_breaker;
pub mod error;
pub mod mcp_v2_extensions;
pub mod models;
pub mod prompt_templates;
pub mod protocol;
pub mod repository;
pub mod validation;
pub mod workspace_setup;

// Re-export commonly used types at the crate root for convenience
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerAction, CircuitState, FailureType};
pub use error::{Result, TaskError};
pub use mcp_v2_extensions::{
    AgentWorkload, CapabilityMatcher, ClaimResult, DiscoverWorkResponse, PrerequisiteAction,
    PriorityCalculator, SimpleKnowledgeEntry, SimpleWorkSession, WorkDiscoveryConfig,
};
pub use models::{
    AgentProfile,
    AgentStatus,
    EventSeverity,
    // MCP v2 New Entity Types
    KnowledgeObject,
    NewTask,
    SystemEvent,
    Task,
    TaskFilter,
    TaskMessage,
    TaskState,
    UpdateTask,
    WorkSession,
    WorkflowDefinition,
};
pub use prompt_templates::{
    generate_enhanced_setup_instructions, AgentContract, CapabilityDefinition, CoordinationRecipe,
    EnhancedPromptBuilder,
};
pub use protocol::{
    ArchiveTaskParams,
    AssignTaskParams,
    ClaimTaskParams,
    CreateMainAiFileParams,
    // Task Messaging Types
    CreateTaskMessageParams,
    CreateTaskParams,
    // MCP v2 Advanced Multi-Agent Types
    DiscoverWorkParams,
    EndWorkSessionParams,
    GetAgenticWorkflowDescriptionParams,
    GetInstructionsForMainAiFileParams,
    // Workspace Setup Types
    GetSetupInstructionsParams,
    GetTaskByCodeParams,
    GetTaskByIdParams,
    GetTaskMessagesParams,
    HealthStatus,
    ListTasksParams,
    ProtocolHandler,
    RegisterAgentParams,
    ReleaseTaskParams,
    SetStateParams,
    StartWorkSessionParams,
    UpdateTaskParams,
    WorkSessionInfo,
};
pub use repository::{
    RepositoryStats, TaskMessageRepository, TaskRepository, WorkspaceContextRepository,
};
pub use validation::TaskValidator;
pub use workspace_setup::{
    AgentRegistration, AgenticWorkflowDescription, AiToolType, GeneratedFileMetadata,
    MainAiFileData, MainAiFileInstructions, PrdDocument, SetupInstructions, WorkspaceContext,
    WorkspaceManifest, WorkspaceSetupConfig, WorkspaceSetupError, WorkspaceSetupResult,
    WorkspaceSetupService,
};

/// Current version of the core crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current crate name
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_crate_constants() {
        assert!(!VERSION.is_empty());
        assert_eq!(CRATE_NAME, "task-core");
    }

    #[test]
    fn test_re_exports() {
        use crate::{TaskError, TaskState};

        // Test that re-exports work
        let state = TaskState::Created;
        assert_eq!(format!("{state}"), "Created");

        let error = TaskError::not_found_id(1);
        assert!(error.is_not_found());
    }
}
