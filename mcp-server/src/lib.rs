//! MCP Server Library
//!
//! This library provides the core functionality for the MCP (Model Context Protocol)
//! task management server. It includes configuration management, database setup,
//! and server initialization.

pub mod config;
pub mod setup;
pub mod telemetry;

pub use config::Config;
pub use setup::{create_repository, create_server, ensure_database_directory, initialize_app};
pub use telemetry::init_telemetry;
