//! Mock implementations and test utilities for the MCP Task Management Server
//!
//! This crate provides comprehensive testing infrastructure including:
//! - Mock implementations of all core traits
//! - Realistic test data generators
//! - Custom assertion helpers
//! - Property-based testing strategies
//! - Contract test helpers

pub mod assertions;
pub mod builders;
pub mod contracts;
pub mod fixtures;
pub mod generators;
pub mod repository;

pub use assertions::*;
pub use builders::*;
pub use contracts::*;
pub use fixtures::*;
pub use generators::*;
pub use repository::MockTaskRepository;
