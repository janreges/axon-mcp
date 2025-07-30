//! Protocol compliance tests for JSON-RPC 2.0
//!
//! Validates that all responses follow the JSON-RPC 2.0 specification

use mcp_protocol::*;
use serde_json::{json, Value};
use task_core::TaskState;

#[test]
fn test_success_response_format() {
    let result = json!({"task_id": 123});
    let id = Some(json!(1));
    
    let response = create_success_response(id, result);
    
    // JSON-RPC 2.0 compliance checks
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["task_id"], 123);
    assert!(response.get("error").is_none());
}

#[test]
fn test_null_response_format() {
    let id = Some(json!(2));
    let response = create_null_response(id);
    
    // JSON-RPC 2.0 compliance checks
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["result"].is_null());
    assert!(response.get("error").is_none());
}

#[test]
fn test_error_response_format() {
    let error = McpError::NotFound("Task 123 not found".to_string());
    let id = Some(json!(3));
    
    let response = error.to_json_rpc_error(id);
    
    // JSON-RPC 2.0 compliance checks
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response.get("result").is_none());
    
    let error_obj = &response["error"];
    assert!(error_obj.is_object());
    assert_eq!(error_obj["code"], -32001);
    assert!(error_obj["message"].is_string());
    assert!(error_obj["message"].as_str().unwrap().contains("Task 123 not found"));
}

#[test]
fn test_error_codes_compliance() {
    // Test all error codes are in the correct range
    let test_cases = vec![
        (McpError::NotFound("test".into()), -32001),
        (McpError::Validation("test".into()), -32002),
        (McpError::DuplicateCode("test".into()), -32003),
        (McpError::InvalidStateTransition("test".into()), -32004),
        (McpError::Database("test".into()), -32005),
        (McpError::Protocol("test".into()), -32006),
        (McpError::Serialization("test".into()), -32007),
    ];
    
    for (error, expected_code) in test_cases {
        let code = error.to_error_code();
        assert_eq!(code, expected_code);
        
        // All codes should be in the implementation-defined range
        assert!(code >= -32099 && code <= -32000, "Error code {} is not in the implementation-defined range", code);
    }
}

#[test]
fn test_task_serialization_format() {
    use chrono::Utc;
    
    let task = task_core::Task {
        id: 42,
        code: "TEST-042".to_string(),
        name: "Test Task".to_string(),
        description: "A test task for serialization".to_string(),
        owner_agent_name: Some("test-agent".to_string()),
        state: TaskState::InProgress,
        inserted_at: Utc::now(),
        done_at: None,
            workflow_definition_id: None,
            workflow_cursor: None,
            priority_score: 5.0,
            parent_task_id: None,
            failure_count: 0,
            required_capabilities: vec![],
            estimated_effort: None,
            confidence_threshold: 0.8,
        };
    
    let serialized = serialize_task_for_mcp(&task).unwrap();
    
    // Check required fields are present
    assert!(serialized.get("id").is_some());
    assert!(serialized.get("code").is_some());
    assert!(serialized.get("name").is_some());
    assert!(serialized.get("description").is_some());
    assert!(serialized.get("owner_agent_name").is_some());
    assert!(serialized.get("state").is_some());
    assert!(serialized.get("inserted_at").is_some());
    assert!(serialized.get("done_at").is_some());
    
    // Check data types
    assert!(serialized["id"].is_i64());
    assert!(serialized["code"].is_string());
    assert!(serialized["name"].is_string());
    assert!(serialized["description"].is_string());
    assert!(serialized["owner_agent_name"].is_string());
    assert!(serialized["state"].is_string());
    assert!(serialized["inserted_at"].is_string());
    assert!(serialized["done_at"].is_null()); // None should serialize to null
    
    // Check timestamp format (should be RFC3339)
    let timestamp_str = serialized["inserted_at"].as_str().unwrap();
    assert!(timestamp_str.contains("T"));
    assert!(timestamp_str.ends_with("Z") || timestamp_str.contains("+"));
}

#[test]
fn test_parameter_deserialization() {
    let params = json!({
        "code": "PARAM-001",
        "name": "Parameter Test",
        "description": "Testing parameter deserialization",
        "owner_agent_name": "param-tester"
    });
    
    let deserialized: Result<CreateTaskParams, _> = deserialize_mcp_params(params);
    assert!(deserialized.is_ok());
    
    let create_params = deserialized.unwrap();
    assert_eq!(create_params.code, "PARAM-001");
    assert_eq!(create_params.name, "Parameter Test");
    assert_eq!(create_params.description, "Testing parameter deserialization");
    assert_eq!(create_params.owner_agent_name.as_deref(), Some("param-tester"));
}

#[test]
fn test_invalid_parameter_deserialization() {
    // Missing required field
    let params = json!({
        "code": "INVALID-001",
        "name": "Invalid Test"
        // Missing description and owner_agent_name
    });
    
    let result: Result<CreateTaskParams, _> = deserialize_mcp_params(params);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), McpError::Serialization(_)));
}

#[test]
fn test_list_params_filter_conversion() {
    let params = ListTasksParams {
        owner_agent_name: Some("test-owner".to_string()),
        state: Some(TaskState::InProgress),
        created_after: Some("2023-01-01T00:00:00Z".to_string()),
        created_before: Some("2023-12-31T23:59:59Z".to_string()),
        completed_after: None,
        completed_before: None,
        limit: Some(10),
    };
    
    let filter = params.to_task_filter().unwrap();
    assert_eq!(filter.owner, Some("test-owner".to_string()));
    assert_eq!(filter.state, Some(TaskState::InProgress));
    assert!(filter.date_from.is_some());
    assert!(filter.date_to.is_some());
}

#[test]
fn test_invalid_datetime_format() {
    let params = ListTasksParams {
        created_after: Some("invalid-date".to_string()),
        ..Default::default()
    };
    
    let result = params.to_task_filter();
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), task_core::TaskError::Validation(_)));
}

#[test]
fn test_response_without_id() {
    // JSON-RPC 2.0 allows null id for notifications
    let result = json!({"success": true});
    let response = create_success_response(None, result);
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["id"].is_null());
    assert_eq!(response["result"]["success"], true);
}

#[test]
fn test_batch_request_compatibility() {
    // While we don't implement batch requests yet, ensure our response format
    // would be compatible with batch responses
    let responses = vec![
        create_success_response(Some(json!(1)), json!({"task_id": 1})),
        create_success_response(Some(json!(2)), json!({"task_id": 2})),
    ];
    
    for (i, response) in responses.iter().enumerate() {
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], i + 1);
        assert!(response.get("result").is_some());
        assert!(response.get("error").is_none());
    }
}

#[test]
fn test_task_state_serialization() {
    let states = vec![
        TaskState::Created,
        TaskState::InProgress,
        TaskState::Blocked,
        TaskState::Review,
        TaskState::Done,
        TaskState::Archived,
    ];
    
    for state in states {
        let serialized = serde_json::to_value(state).unwrap();
        assert!(serialized.is_string());
        
        // Should be able to deserialize back
        let deserialized: TaskState = serde_json::from_value(serialized).unwrap();
        assert_eq!(deserialized, state);
    }
}

#[test]
fn test_error_message_format() {
    use task_core::TaskError;
    
    let core_error = TaskError::InvalidStateTransition(TaskState::Created, TaskState::Done);
    let mcp_error = McpError::from(core_error);
    
    let response = mcp_error.to_json_rpc_error(Some(json!(1)));
    let error_message = response["error"]["message"].as_str().unwrap();
    
    // Error message should be descriptive and include state names
    assert!(error_message.contains("Created"));
    assert!(error_message.contains("Done"));
    assert!(error_message.contains("transition"));
}