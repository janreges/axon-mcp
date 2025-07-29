//! Performance tests for MCP protocol
//!
//! Ensures response times meet the <100ms requirement

use std::sync::Arc;
use std::time::Instant;
use mcp_protocol::*;
use task_core::{TaskRepository, Task, NewTask, UpdateTask, TaskState, TaskFilter, RepositoryStats};
use task_core::error::{Result, TaskError};
use async_trait::async_trait;
use chrono::Utc;

/// Fast mock repository for performance testing
#[derive(Clone)]
struct FastMockRepository;

#[async_trait]
impl TaskRepository for FastMockRepository {
    async fn create(&self, task: NewTask) -> Result<Task> {
        Ok(Task {
            id: 1,
            code: task.code,
            name: task.name,
            description: task.description,
            owner_agent_name: task.owner_agent_name,
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        })
    }
    
    async fn update(&self, id: i32, updates: task_core::UpdateTask) -> Result<Task> {
        Ok(Task {
            id,
            code: "TEST-001".to_string(),
            name: updates.name.unwrap_or_else(|| "Test Task".to_string()),
            description: updates.description.unwrap_or_else(|| "Test description".to_string()),
            owner_agent_name: updates.owner_agent_name.unwrap_or_else(|| "test-agent".to_string()),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        })
    }
    
    async fn set_state(&self, id: i32, state: TaskState) -> Result<Task> {
        Ok(Task {
            id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state,
            inserted_at: Utc::now(),
            done_at: if state == TaskState::Done { Some(Utc::now()) } else { None },
        })
    }
    
    async fn get_by_id(&self, id: i32) -> Result<Option<Task>> {
        Ok(Some(Task {
            id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        }))
    }
    
    async fn get_by_code(&self, _code: &str) -> Result<Option<Task>> {
        Ok(Some(Task {
            id: 1,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        }))
    }
    
    async fn list(&self, _filter: TaskFilter) -> Result<Vec<Task>> {
        Ok(vec![
            Task {
                id: 1,
                code: "TEST-001".to_string(),
                name: "Test Task 1".to_string(),
                description: "Test description 1".to_string(),
                owner_agent_name: "test-agent".to_string(),
                state: TaskState::Created,
                inserted_at: Utc::now(),
                done_at: None,
            },
            Task {
                id: 2,
                code: "TEST-002".to_string(),
                name: "Test Task 2".to_string(),
                description: "Test description 2".to_string(),
                owner_agent_name: "test-agent".to_string(),
                state: TaskState::InProgress,
                inserted_at: Utc::now(),
                done_at: None,
            },
        ])
    }
    
    async fn assign(&self, id: i32, new_owner: &str) -> Result<Task> {
        Ok(Task {
            id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: new_owner.to_string(),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        })
    }
    
    async fn archive(&self, id: i32) -> Result<Task> {
        Ok(Task {
            id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Archived,
            inserted_at: Utc::now(),
            done_at: None,
        })
    }
    
    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
    
    async fn get_stats(&self) -> Result<RepositoryStats> {
        Ok(RepositoryStats::default())
    }
}

async fn measure_operation<F, Fut, T>(operation: F) -> (T, std::time::Duration)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = operation().await;
    let duration = start.elapsed();
    (result, duration)
}

#[tokio::test]
async fn test_create_task_performance() {
    let repository = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository);
    
    let params = CreateTaskParams {
        code: "PERF-001".to_string(),
        name: "Performance Test".to_string(),
        description: "Testing create task performance".to_string(),
        owner_agent_name: "perf-agent".to_string(),
    };
    
    let (result, duration) = measure_operation(|| handler.create_task(params.clone())).await;
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 100, "Create task took {}ms, should be <100ms", duration.as_millis());
    println!("Create task performance: {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_get_task_performance() {
    let repository = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository);
    
    let params = GetTaskByIdParams { id: 1 };
    
    let (result, duration) = measure_operation(|| handler.get_task_by_id(params.clone())).await;
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 100, "Get task took {}ms, should be <100ms", duration.as_millis());
    println!("Get task performance: {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_list_tasks_performance() {
    let repository = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository);
    
    let params = ListTasksParams::default();
    
    let (result, duration) = measure_operation(|| handler.list_tasks(params.clone())).await;
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 100, "List tasks took {}ms, should be <100ms", duration.as_millis());
    println!("List tasks performance: {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_update_task_performance() {
    let repository = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository);
    
    let params = UpdateTaskParams {
        id: 1,  
        update_data: UpdateTask {
            name: Some("Updated Name".to_string()),
            description: None,
            owner_agent_name: None,
        },
    };
    
    let (result, duration) = measure_operation(|| handler.update_task(params.clone())).await;
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 100, "Update task took {}ms, should be <100ms", duration.as_millis());
    println!("Update task performance: {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_state_transition_performance() {
    let repository = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository);
    
    let params = SetStateParams {
        id: 1,
        state: TaskState::InProgress,
    };
    
    let (result, duration) = measure_operation(|| handler.set_task_state(params.clone())).await;
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 100, "Set state took {}ms, should be <100ms", duration.as_millis());
    println!("Set state performance: {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_serialization_performance() {
    use chrono::Utc;
    
    let task = Task {
        id: 1,
        code: "PERF-SERIAL".to_string(),
        name: "Serialization Performance Test".to_string(),
        description: "Testing task serialization performance".to_string(),
        owner_agent_name: "perf-agent".to_string(),
        state: TaskState::InProgress,
        inserted_at: Utc::now(),
        done_at: None,
    };
    
    let start = Instant::now();
    for _ in 0..1000 {
        let _serialized = serialize_task_for_mcp(&task).unwrap();
    }
    let duration = start.elapsed();
    
    let avg_duration = duration / 1000;
    assert!(avg_duration.as_micros() < 1000, "Average serialization took {}μs, should be <1000μs", avg_duration.as_micros());
    println!("Average serialization performance: {}μs", avg_duration.as_micros());
}

#[tokio::test]
async fn test_concurrent_operations_performance() {
    let repository = Arc::new(FastMockRepository);
    let handler = Arc::new(McpTaskHandler::new(repository));
    
    let start = Instant::now();
    
    let mut handles = Vec::new();
    
    // Spawn 10 concurrent operations
    for i in 0..10 {
        let handler_clone = handler.clone();
        let handle = tokio::spawn(async move {
            let params = CreateTaskParams {
                code: format!("CONCURRENT-{:03}", i),
                name: format!("Concurrent Test {}", i),
                description: "Concurrent performance test".to_string(),
                owner_agent_name: "concurrent-agent".to_string(),
            };
            
            handler_clone.create_task(params).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
    
    let total_duration = start.elapsed();
    
    // All 10 operations should complete well within 100ms
    assert!(total_duration.as_millis() < 100, "10 concurrent operations took {}ms, should be <100ms", total_duration.as_millis());
    println!("10 concurrent operations performance: {}ms", total_duration.as_millis());
}

#[test]
fn test_error_conversion_performance() {
    use task_core::TaskError;
    
    let core_errors = vec![
        TaskError::NotFound("test".to_string()),
        TaskError::Validation("test".to_string()),
        TaskError::DuplicateCode("test".to_string()),
        TaskError::InvalidStateTransition(TaskState::Created, TaskState::Done),
        TaskError::Database("test".to_string()),
        TaskError::Protocol("test".to_string()),
    ];
    
    let start = Instant::now();
    
    for _ in 0..1000 {
        for error in &core_errors {
            let mcp_error = McpError::from(error.clone());
            let _json_error = mcp_error.to_json_rpc_error(Some(serde_json::json!(1)));
        }
    }
    
    let duration = start.elapsed();
    let avg_duration = duration / (1000 * core_errors.len() as u32);
    
    assert!(avg_duration.as_micros() < 100, "Average error conversion took {}μs, should be <100μs", avg_duration.as_micros());
    println!("Average error conversion performance: {}μs", avg_duration.as_micros());
}