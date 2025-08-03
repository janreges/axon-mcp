//! MCP Protocol Implementation
//!
//! This crate provides the Model Context Protocol (MCP) server implementation
//! using Server-Sent Events (SSE) transport for task management operations.
//!
//! # Overview
//!
//! The mcp-protocol crate implements the bridge between core business logic
//! and MCP clients using Server-Sent Events (SSE) transport. It provides:
//!
//! - JSON-RPC 2.0 compliant protocol handling
//! - SSE transport layer for real-time communication
//! - Error mapping from core errors to MCP error codes
//! - Task serialization for MCP responses
//!
//! # Usage
//!
//! ```no_run
//! use mcp_protocol::{McpServer, McpTaskHandler};
//! use std::sync::Arc;
//!
//! async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
//!     // In real usage, you would use database::SqliteTaskRepository
//!     // let repository = Arc::new(database::SqliteTaskRepository::new("tasks.db").await?);
//!     # use task_core::{TaskRepository, TaskMessageRepository, WorkspaceContextRepository, Task, TaskMessage, NewTask, UpdateTask, TaskFilter, TaskState, RepositoryStats};
//!     # use task_core::error::Result;
//!     # use async_trait::async_trait;
//!     # struct MockRepo;
//!     # struct MockMessageRepo;
//!     # struct MockWorkspaceRepo;
//!     # #[async_trait]
//!     # impl TaskRepository for MockRepo {
//!     #     async fn create(&self, _task: NewTask) -> Result<Task> { unimplemented!() }
//!     #     async fn update(&self, _id: i32, _updates: UpdateTask) -> Result<Task> { unimplemented!() }
//!     #     async fn set_state(&self, _id: i32, _state: TaskState) -> Result<Task> { unimplemented!() }
//!     #     async fn get_by_id(&self, _id: i32) -> Result<Option<Task>> { unimplemented!() }
//!     #     async fn get_by_code(&self, _code: &str) -> Result<Option<Task>> { unimplemented!() }
//!     #     async fn list(&self, _filter: TaskFilter) -> Result<Vec<Task>> { unimplemented!() }
//!     #     async fn assign(&self, _id: i32, _new_owner: &str) -> Result<Task> { unimplemented!() }
//!     #     async fn archive(&self, _id: i32) -> Result<Task> { unimplemented!() }
//!     #     async fn health_check(&self) -> Result<()> { unimplemented!() }
//!     #     async fn get_stats(&self) -> Result<RepositoryStats> { unimplemented!() }
//!     #     async fn discover_work(&self, _agent_name: &str, _capabilities: &[String], _max_tasks: u32) -> Result<Vec<Task>> { unimplemented!() }
//!     #     async fn claim_task(&self, _task_id: i32, _agent_name: &str) -> Result<Task> { unimplemented!() }
//!     #     async fn release_task(&self, _task_id: i32, _agent_name: &str) -> Result<Task> { unimplemented!() }
//!     #     async fn start_work_session(&self, _task_id: i32, _agent_name: &str) -> Result<i32> { unimplemented!() }
//!     #     async fn end_work_session(&self, _session_id: i32, _notes: Option<String>, _productivity_score: Option<f64>) -> Result<()> { unimplemented!() }
//!     # }
//!     # #[async_trait]
//!     # impl TaskMessageRepository for MockMessageRepo {
//!     #     async fn create_message(&self, _task_code: &str, _author_agent_name: &str, _target_agent_name: Option<&str>, _message_type: &str, _content: &str, _reply_to_message_id: Option<i32>) -> Result<TaskMessage> { unimplemented!() }
//!     #     async fn get_messages(&self, _task_code: &str, _author_agent_name: Option<&str>, _target_agent_name: Option<&str>, _message_type: Option<&str>, _reply_to_message_id: Option<i32>, _limit: Option<u32>) -> Result<Vec<TaskMessage>> { unimplemented!() }
//!     #     async fn get_message_by_id(&self, _message_id: i32) -> Result<Option<TaskMessage>> { unimplemented!() }
//!     # }
//!     # #[async_trait]
//!     # impl WorkspaceContextRepository for MockWorkspaceRepo {
//!     #     async fn create(&self, _context: task_core::workspace_setup::WorkspaceContext) -> Result<task_core::workspace_setup::WorkspaceContext> { unimplemented!() }
//!     #     async fn get_by_id(&self, _workspace_id: &str) -> Result<Option<task_core::workspace_setup::WorkspaceContext>> { unimplemented!() }
//!     #     async fn update(&self, _context: task_core::workspace_setup::WorkspaceContext) -> Result<task_core::workspace_setup::WorkspaceContext> { unimplemented!() }
//!     #     async fn delete(&self, _workspace_id: &str) -> Result<()> { unimplemented!() }
//!     #     async fn health_check(&self) -> Result<()> { unimplemented!() }
//!     # }
//!     let repository = Arc::new(MockRepo);
//!     let message_repository = Arc::new(MockMessageRepo);
//!     let workspace_repository = Arc::new(MockWorkspaceRepo);
//!     let server = McpServer::new(repository, message_repository, workspace_repository);
//!     server.serve("127.0.0.1:3000").await?;
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod handler;
pub mod serialization;
pub mod server;

// Re-export key types for easier usage
pub use error::*;
pub use handler::McpTaskHandler;
pub use serialization::*;
pub use server::McpServer;

// Re-export core types for external consumers
pub use task_core::{
    ArchiveTaskParams, AssignTaskParams, CreateTaskParams, GetTaskByCodeParams, GetTaskByIdParams,
    HealthStatus, ListTasksParams, NewTask, ProtocolHandler, SetStateParams, Task, TaskFilter,
    TaskRepository, TaskState, UpdateTask, UpdateTaskParams,
};
