//! Performance tests for MCP protocol
//!
//! Ensures response times meet the <100ms requirement

use std::sync::Arc;
use std::time::Instant;
use mcp_protocol::*;
use task_core::{TaskRepository, TaskMessageRepository, WorkspaceContextRepository, Task, NewTask, UpdateTask, TaskState, TaskFilter, RepositoryStats, TaskMessage};
use task_core::workspace_setup::WorkspaceContext;
use task_core::error::Result;
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
            workflow_definition_id: task.workflow_definition_id,
            workflow_cursor: None, // NewTask doesn't have workflow_cursor
            priority_score: task.priority_score,
            parent_task_id: task.parent_task_id,
            failure_count: 0,
            required_capabilities: task.required_capabilities,
            estimated_effort: task.estimated_effort,
            confidence_threshold: task.confidence_threshold,
        })
    }
    
    async fn update(&self, id: i32, updates: task_core::UpdateTask) -> Result<Task> {
        Ok(Task {
            id,
            code: "TEST-001".to_string(),
            name: updates.name.unwrap_or_else(|| "Test Task".to_string()),
            description: updates.description.unwrap_or_else(|| "Test description".to_string()),
            owner_agent_name: Some(updates.owner_agent_name.unwrap_or_else(|| "test-agent".to_string())),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
            workflow_definition_id: updates.workflow_definition_id.flatten(),
            workflow_cursor: updates.workflow_cursor.flatten(),
            priority_score: updates.priority_score.unwrap_or(5.0),
            parent_task_id: updates.parent_task_id.flatten(),
            failure_count: 0,
            required_capabilities: updates.required_capabilities.unwrap_or_default(),
            estimated_effort: updates.estimated_effort.flatten(),
            confidence_threshold: updates.confidence_threshold.unwrap_or(0.8),
        })
    }
    
    async fn set_state(&self, id: i32, state: TaskState) -> Result<Task> {
        Ok(Task {
            id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: Some("test-agent".to_string()),
            state,
            inserted_at: Utc::now(),
            done_at: if state == TaskState::Done { Some(Utc::now()) } else { None },
            workflow_definition_id: None,
            workflow_cursor: None,
            priority_score: 5.0,
            parent_task_id: None,
            failure_count: 0,
            required_capabilities: vec![],
            estimated_effort: None,
            confidence_threshold: 0.8,
        })
    }
    
    async fn get_by_id(&self, id: i32) -> Result<Option<Task>> {
        Ok(Some(Task {
            id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: Some("test-agent".to_string()),
            state: TaskState::Created,
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
        }))
    }
    
    async fn get_by_code(&self, _code: &str) -> Result<Option<Task>> {
        Ok(Some(Task {
            id: 1,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: Some("test-agent".to_string()),
            state: TaskState::Created,
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
        }))
    }
    
    async fn list(&self, _filter: TaskFilter) -> Result<Vec<Task>> {
        Ok(vec![
            Task {
                id: 1,
                code: "TEST-001".to_string(),
                name: "Test Task 1".to_string(),
                description: "Test description 1".to_string(),
                owner_agent_name: Some("test-agent".to_string()),
                state: TaskState::Created,
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
        },
            Task {
                id: 2,
                code: "TEST-002".to_string(),
                name: "Test Task 2".to_string(),
                description: "Test description 2".to_string(),
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
        },
        ])
    }
    
    async fn assign(&self, id: i32, new_owner: &str) -> Result<Task> {
        Ok(Task {
            id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: Some(new_owner.to_string()),
            state: TaskState::Created,
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
        })
    }
    
    async fn archive(&self, id: i32) -> Result<Task> {
        Ok(Task {
            id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: Some("test-agent".to_string()),
            state: TaskState::Archived,
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
        })
    }
    
    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
    
    async fn get_stats(&self) -> Result<RepositoryStats> {
        Ok(RepositoryStats::default())
    }
    
    async fn discover_work(&self, _agent_name: &str, _capabilities: &[String], _max_tasks: u32) -> Result<Vec<Task>> {
        Ok(vec![])
    }
    
    async fn claim_task(&self, task_id: i32, agent_name: &str) -> Result<Task> {
        Ok(Task {
            id: task_id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: Some(agent_name.to_string()),
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
        })
    }
    
    async fn release_task(&self, task_id: i32, agent_name: &str) -> Result<Task> {
        Ok(Task {
            id: task_id,
            code: "TEST-001".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: Some(agent_name.to_string()),
            state: TaskState::Created,
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
        })
    }
    
    async fn start_work_session(&self, task_id: i32, _agent_name: &str) -> Result<i32> {
        Ok(task_id * 100) // Mock session ID
    }
    
    async fn end_work_session(&self, _session_id: i32, _notes: Option<String>, _productivity_score: Option<f64>) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl TaskMessageRepository for FastMockRepository {
    async fn create_message(
        &self,
        task_code: &str,
        author_agent_name: &str,
        target_agent_name: Option<&str>,
        message_type: &str,
        content: &str,
        reply_to_message_id: Option<i32>,
    ) -> Result<TaskMessage> {
        Ok(TaskMessage {
            id: 1,
            task_code: task_code.to_string(),
            author_agent_name: author_agent_name.to_string(),
            target_agent_name: target_agent_name.map(|s| s.to_string()),
            message_type: message_type.to_string(),
            created_at: Utc::now(),
            content: content.to_string(),
            reply_to_message_id,
        })
    }
    
    async fn get_messages(
        &self,
        _task_code: &str,
        _author_agent_name: Option<&str>,
        _target_agent_name: Option<&str>,
        _message_type: Option<&str>,
        _reply_to_message_id: Option<i32>,
        _limit: Option<u32>,
    ) -> Result<Vec<TaskMessage>> {
        Ok(vec![])
    }
    
    async fn get_message_by_id(&self, _message_id: i32) -> Result<Option<TaskMessage>> {
        Ok(None)
    }
}

/// Fast mock workspace context repository for performance testing
#[async_trait]
impl WorkspaceContextRepository for FastMockRepository {
    async fn create(&self, context: WorkspaceContext) -> Result<WorkspaceContext> {
        Ok(context)
    }
    
    async fn get_by_id(&self, _workspace_id: &str) -> Result<Option<WorkspaceContext>> {
        Ok(None)
    }
    
    async fn update(&self, context: WorkspaceContext) -> Result<WorkspaceContext> {
        Ok(context)
    }
    
    async fn delete(&self, _workspace_id: &str) -> Result<()> {
        Ok(())
    }
    
    async fn health_check(&self) -> Result<()> {
        Ok(())
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
    let workspace_repo = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo);
    
    let params = CreateTaskParams {
        code: "PERF-001".to_string(),
        name: "Performance Test".to_string(),
        description: "Testing create task performance".to_string(),
        owner_agent_name: Some("perf-agent".to_string()),
        confidence_threshold: 0.8,
        estimated_effort: None,
        parent_task_id: None,
        required_capabilities: vec![],
        priority_score: 5.0,
        workflow_definition_id: None,
    };
    
    let (result, duration) = measure_operation(|| handler.create_task(params.clone())).await;
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 100, "Create task took {}ms, should be <100ms", duration.as_millis());
    println!("Create task performance: {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_get_task_performance() {
    let repository = Arc::new(FastMockRepository);
    let workspace_repo = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo);
    
    let params = GetTaskByIdParams { id: 1 };
    
    let (result, duration) = measure_operation(|| handler.get_task_by_id(params.clone())).await;
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 100, "Get task took {}ms, should be <100ms", duration.as_millis());
    println!("Get task performance: {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_list_tasks_performance() {
    let repository = Arc::new(FastMockRepository);
    let workspace_repo = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo);
    
    let params = ListTasksParams::default();
    
    let (result, duration) = measure_operation(|| handler.list_tasks(params.clone())).await;
    
    assert!(result.is_ok());
    assert!(duration.as_millis() < 100, "List tasks took {}ms, should be <100ms", duration.as_millis());
    println!("List tasks performance: {}ms", duration.as_millis());
}

#[tokio::test]
async fn test_update_task_performance() {
    let repository = Arc::new(FastMockRepository);
    let workspace_repo = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo);
    
    let params = UpdateTaskParams {
        id: 1,  
        update_data: UpdateTask {
            name: Some("Updated Name".to_string()),
            description: None,
            owner_agent_name: None,
            workflow_definition_id: None,
            workflow_cursor: None,
            priority_score: Some(5.0),
            parent_task_id: None,
            required_capabilities: None,
            estimated_effort: None,
            confidence_threshold: Some(0.8),
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
    let workspace_repo = Arc::new(FastMockRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo);
    
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
        owner_agent_name: Some("perf-agent".to_string()),
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
    let workspace_repo = Arc::new(FastMockRepository);
    let handler = Arc::new(McpTaskHandler::new(repository.clone(), repository, workspace_repo));
    
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
                owner_agent_name: Some("concurrent-agent".to_string()),
                confidence_threshold: 0.8,
                estimated_effort: None,
                parent_task_id: None,
                required_capabilities: vec![],
                priority_score: 5.0,
                workflow_definition_id: None,
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