//! Error handling for MCP protocol
//!
//! Maps internal task errors to MCP-compliant JSON-RPC error codes.

use ::task_core::TaskError;
use serde_json::{json, Value};
use thiserror::Error;

/// MCP protocol errors
#[derive(Error, Debug)]
pub enum McpError {
    #[error("Task not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Duplicate task code: {0}")]
    DuplicateCode(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl McpError {
    /// Convert to JSON-RPC error code
    pub fn to_error_code(&self) -> i32 {
        match self {
            McpError::NotFound(_) => -32001,
            McpError::Validation(_) => -32002,
            McpError::DuplicateCode(_) => -32003,
            McpError::InvalidStateTransition(_) => -32004,
            McpError::Database(_) => -32005,
            McpError::Protocol(_) => -32006,
            McpError::Serialization(_) => -32007,
        }
    }

    /// Convert to JSON-RPC error response
    pub fn to_json_rpc_error(&self, id: Option<Value>) -> Value {
        json!({
            "jsonrpc": "2.0",
            "error": {
                "code": self.to_error_code(),
                "message": self.to_string()
            },
            "id": id
        })
    }
}

/// Convert from TaskError to McpError
impl From<TaskError> for McpError {
    fn from(err: TaskError) -> Self {
        match err {
            TaskError::NotFound(msg) => McpError::NotFound(msg),
            TaskError::Validation(msg) => McpError::Validation(msg),
            TaskError::DuplicateCode(code) => McpError::DuplicateCode(code),
            TaskError::InvalidStateTransition(from, to) => {
                McpError::InvalidStateTransition(format!("Cannot transition from {from} to {to}"))
            }
            TaskError::Database(msg) => McpError::Database(msg),
            TaskError::Protocol(msg) => McpError::Protocol(msg),
            TaskError::Configuration(msg) => {
                McpError::Protocol(format!("Configuration error: {msg}"))
            }
            TaskError::Internal(msg) => McpError::Protocol(format!("Internal error: {msg}")),
            TaskError::AlreadyClaimed(task_id, owner) => {
                McpError::Validation(format!("Task {task_id} is already claimed by {owner}"))
            }
            TaskError::NotOwned(agent, task_id) => {
                McpError::Validation(format!("Agent {agent} does not own task {task_id}"))
            }
            TaskError::InsufficientCapabilities(agent, required) => McpError::Validation(format!(
                "Agent {agent} lacks required capabilities: {required:?}"
            )),
            TaskError::SessionNotFound(session_id) => {
                McpError::NotFound(format!("Work session {session_id} not found"))
            }
            TaskError::CircuitBreakerOpen(agent) => {
                McpError::Protocol(format!("Circuit breaker open for agent {agent}"))
            }
            TaskError::UnknownAgent(agent) => McpError::NotFound(format!("Unknown agent: {agent}")),
            TaskError::Conflict(msg) => McpError::Validation(format!("Conflict: {msg}")),
            TaskError::Serialization(msg) => {
                McpError::Protocol(format!("Serialization error: {msg}"))
            }
            TaskError::Deserialization(msg) => {
                McpError::Protocol(format!("Deserialization error: {msg}"))
            }
            TaskError::DuplicateKey(key) => McpError::DuplicateCode(key),
            TaskError::UnsupportedAiTool(tool) => {
                McpError::Validation(format!("Unsupported AI tool: {tool}"))
            }
            TaskError::UnsupportedOperation(op) => {
                McpError::Validation(format!("Unsupported operation: {op}"))
            }
        }
    }
}

/// Convert from anyhow::Error to McpError
impl From<anyhow::Error> for McpError {
    fn from(err: anyhow::Error) -> Self {
        // Try to downcast to TaskError first
        if let Some(task_error) = err.downcast_ref::<TaskError>() {
            return Self::from(task_error.clone());
        }

        // Check if it's a serialization error
        let error_msg = err.to_string();
        if error_msg.contains("serialize")
            || error_msg.contains("deserialize")
            || error_msg.contains("JSON")
        {
            McpError::Serialization(error_msg)
        } else if error_msg.contains("parse") || error_msg.contains("invalid") {
            McpError::Validation(error_msg)
        } else {
            // Default to protocol error for other anyhow errors
            McpError::Protocol(error_msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(McpError::NotFound("123".into()).to_error_code(), -32001);
        assert_eq!(
            McpError::Validation("invalid".into()).to_error_code(),
            -32002
        );
        assert_eq!(
            McpError::DuplicateCode("TASK-001".into()).to_error_code(),
            -32003
        );
        assert_eq!(
            McpError::InvalidStateTransition("Invalid".into()).to_error_code(),
            -32004
        );
        assert_eq!(
            McpError::Database("conn failed".into()).to_error_code(),
            -32005
        );
        assert_eq!(
            McpError::Protocol("bad request".into()).to_error_code(),
            -32006
        );
    }

    #[test]
    fn test_json_rpc_error() {
        let error = McpError::NotFound("123".into());
        let json_error = error.to_json_rpc_error(Some(json!(1)));

        assert_eq!(json_error["jsonrpc"], "2.0");
        assert_eq!(json_error["error"]["code"], -32001);
        assert_eq!(json_error["id"], 1);
    }
}
