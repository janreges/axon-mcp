//! Database crate for the MCP Task Management System
//! 
//! This crate provides SQLite implementation of the TaskRepository trait,
//! offering high-performance task persistence with connection pooling,
//! prepared statements, and comprehensive error handling.
//! 
//! # Features
//! 
//! - SQLite database support with WAL mode for better concurrency
//! - Database migrations with proper schema management
//! - Connection pooling for optimal performance
//! - Comprehensive error handling and mapping
//! - Full test coverage with in-memory database support
//! 
//! # Usage
//! 
//! ```rust
//! use database::SqliteTaskRepository;
//! use task_core::repository::TaskRepository;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create repository (in-memory for testing)
//!     let repo = SqliteTaskRepository::new(":memory:").await?;
//!     
//!     // Run migrations
//!     repo.migrate().await?;
//!     
//!     // Repository is ready to use
//!     let health = repo.health_check().await?;
//!     println!("Database is healthy!");
//!     
//!     Ok(())
//! }
//! ```

mod common;
mod sqlite;

pub use sqlite::SqliteTaskRepository;

// Re-export commonly used types from task-core for convenience
pub use task_core::{
    error::{Result, TaskError},
    models::{Task, TaskState, TaskFilter, NewTask, UpdateTask, TaskMessage},
    repository::{TaskRepository, TaskMessageRepository, RepositoryStats},
};
