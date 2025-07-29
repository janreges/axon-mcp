//! Serialization utilities for MCP protocol
//! 
//! Handles conversion between internal Task types and MCP JSON format.

use serde::{Deserialize};
use serde_json::{json, Value};
use ::task_core::Task;
use crate::error::McpError;

/// Serialize task for MCP response
pub fn serialize_task_for_mcp(task: &Task) -> Result<Value, McpError> {
    let task_json = json!({
        "id": task.id,
        "code": task.code,
        "name": task.name,
        "description": task.description,
        "owner_agent_name": task.owner_agent_name,
        "state": task.state,
        "inserted_at": task.inserted_at.to_rfc3339(),
        "done_at": task.done_at.map(|dt| dt.to_rfc3339())
    });
    
    Ok(task_json)
}

/// Deserialize MCP parameters
pub fn deserialize_mcp_params<T>(params: Value) -> Result<T, McpError> 
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(params)
        .map_err(|e| McpError::Serialization(e.to_string()))
}

/// Create successful MCP response
pub fn create_success_response(id: Option<Value>, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "result": result,
        "id": id
    })
}

/// Create null result for not found cases
pub fn create_null_response(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0", 
        "result": null,
        "id": id
    })
}

// Re-export parameter types from core for convenience
pub use ::task_core::{
    CreateTaskParams, UpdateTaskParams, SetStateParams,
    GetTaskByIdParams, GetTaskByCodeParams, ListTasksParams,
    AssignTaskParams, ArchiveTaskParams,
};

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_deserialize_create_params() {
        let params = json!({
            "code": "TASK-001",
            "name": "Test Task",
            "description": "A test task",
            "owner_agent_name": "test-agent"
        });
        
        let result: Result<CreateTaskParams, _> = deserialize_mcp_params(params);
        assert!(result.is_ok());
        
        let params = result.unwrap();
        assert_eq!(params.code, "TASK-001");
        assert_eq!(params.name, "Test Task");
    }
    
    #[test]
    fn test_success_response() {
        let response = create_success_response(Some(json!(1)), json!({"success": true}));
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["success"], true);
    }
}