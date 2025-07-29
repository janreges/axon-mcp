use database::{SqliteTaskRepository, TaskRepository, NewTask, UpdateTask, TaskFilter, TaskState, TaskError};
use std::time::Duration;
use tokio::time::Instant;

async fn create_test_repository() -> SqliteTaskRepository {
    // Use a unique timestamp-based name for each test to avoid conflicts
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let thread_id = std::thread::current().id();
    let db_name = format!(":memory:test_{}_{:?}", timestamp, thread_id);
    let repo = SqliteTaskRepository::new(&db_name).await.unwrap();
    repo.migrate().await.unwrap();
    repo
}

#[tokio::test]
async fn test_repository_creation_and_health() {
    let repo = create_test_repository().await;
    
    // Health check should pass
    assert!(repo.health_check().await.is_ok());
    
    // Stats should work with empty database
    let stats = repo.get_stats().await.unwrap();
    assert_eq!(stats.total_tasks, 0);
    assert!(stats.tasks_by_state.is_empty());
    assert!(stats.tasks_by_owner.is_empty());
}

#[tokio::test]
async fn test_full_task_lifecycle() {
    let repo = create_test_repository().await;
    
    // Create a new task
    let new_task = NewTask {
        code: "LIFECYCLE-001".to_string(),
        name: "Test Lifecycle".to_string(),
        description: "Complete task lifecycle test".to_string(),
        owner_agent_name: "test-agent".to_string(),
    };
    
    let mut task = repo.create(new_task).await.unwrap();
    assert_eq!(task.state, TaskState::Created);
    assert!(task.done_at.is_none());
    
    // Move through states
    task = repo.set_state(task.id, TaskState::InProgress).await.unwrap();
    assert_eq!(task.state, TaskState::InProgress);
    
    task = repo.set_state(task.id, TaskState::Review).await.unwrap();
    assert_eq!(task.state, TaskState::Review);
    
    task = repo.set_state(task.id, TaskState::Done).await.unwrap();
    assert_eq!(task.state, TaskState::Done);
    assert!(task.done_at.is_some());
    
    // Archive the task
    task = repo.archive(task.id).await.unwrap();
    assert_eq!(task.state, TaskState::Archived);
    
    // Verify we can still retrieve archived task
    let retrieved = repo.get_by_id(task.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().state, TaskState::Archived);
}

#[tokio::test]
async fn test_task_updates() {
    let repo = create_test_repository().await;
    
    let new_task = NewTask {
        code: "UPDATE-001".to_string(),
        name: "Original Name".to_string(),
        description: "Original description".to_string(),
        owner_agent_name: "original-agent".to_string(),
    };
    
    let task = repo.create(new_task).await.unwrap();
    
    // Update all fields
    let updates = UpdateTask {
        name: Some("Updated Name".to_string()),
        description: Some("Updated description".to_string()),
        owner_agent_name: Some("updated-agent".to_string()),
    };
    
    let updated_task = repo.update(task.id, updates).await.unwrap();
    assert_eq!(updated_task.name, "Updated Name");
    assert_eq!(updated_task.description, "Updated description");
    assert_eq!(updated_task.owner_agent_name, "updated-agent");
    assert_eq!(updated_task.code, "UPDATE-001"); // Code should not change
    
    // Update partial fields
    let partial_updates = UpdateTask {
        name: Some("Partially Updated".to_string()),
        description: None,
        owner_agent_name: None,
    };
    
    let partially_updated = repo.update(task.id, partial_updates).await.unwrap();
    assert_eq!(partially_updated.name, "Partially Updated");
    assert_eq!(partially_updated.description, "Updated description"); // Should remain unchanged
    assert_eq!(partially_updated.owner_agent_name, "updated-agent"); // Should remain unchanged
}

#[tokio::test]
async fn test_task_assignment() {
    let repo = create_test_repository().await;
    
    let new_task = NewTask {
        code: "ASSIGN-001".to_string(),
        name: "Assignment Test".to_string(),
        description: "Test task assignment".to_string(),
        owner_agent_name: "original-agent".to_string(),
    };
    
    let task = repo.create(new_task).await.unwrap();
    
    // Assign to new agent
    let assigned_task = repo.assign(task.id, "new-agent").await.unwrap();
    assert_eq!(assigned_task.owner_agent_name, "new-agent");
    
    // Verify assignment persisted
    let retrieved = repo.get_by_id(task.id).await.unwrap().unwrap();
    assert_eq!(retrieved.owner_agent_name, "new-agent");
}

#[tokio::test]
async fn test_task_retrieval() {
    let repo = create_test_repository().await;
    
    let new_task = NewTask {
        code: "RETRIEVE-001".to_string(),
        name: "Retrieval Test".to_string(),
        description: "Test task retrieval".to_string(),
        owner_agent_name: "test-agent".to_string(),
    };
    
    let created_task = repo.create(new_task).await.unwrap();
    
    // Test get by ID
    let by_id = repo.get_by_id(created_task.id).await.unwrap();
    assert!(by_id.is_some());
    assert_eq!(by_id.unwrap().code, "RETRIEVE-001");
    
    // Test get by code
    let by_code = repo.get_by_code("RETRIEVE-001").await.unwrap();
    assert!(by_code.is_some());
    assert_eq!(by_code.unwrap().id, created_task.id);
    
    // Test non-existent lookups
    assert!(repo.get_by_id(99999).await.unwrap().is_none());
    assert!(repo.get_by_code("NON-EXISTENT").await.unwrap().is_none());
}

#[tokio::test]
async fn test_task_filtering() {
    let repo = create_test_repository().await;
    
    // Create multiple tasks with different owners and states
    let tasks = vec![
        NewTask {
            code: "FILTER-001".to_string(),
            name: "Agent 1 Task 1".to_string(),
            description: "Task for agent 1".to_string(),
            owner_agent_name: "agent-1".to_string(),
        },
        NewTask {
            code: "FILTER-002".to_string(),
            name: "Agent 1 Task 2".to_string(),
            description: "Another task for agent 1".to_string(),
            owner_agent_name: "agent-1".to_string(),
        },
        NewTask {
            code: "FILTER-003".to_string(),
            name: "Agent 2 Task".to_string(),
            description: "Task for agent 2".to_string(),
            owner_agent_name: "agent-2".to_string(),
        },
    ];
    
    let mut created_tasks = Vec::new();
    for task in tasks {
        created_tasks.push(repo.create(task).await.unwrap());
    }
    
    // Move one task to InProgress
    repo.set_state(created_tasks[0].id, TaskState::InProgress).await.unwrap();
    
    // Test filtering by owner
    let agent1_tasks = repo.list(TaskFilter {
        owner: Some("agent-1".to_string()),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(agent1_tasks.len(), 2);
    
    let agent2_tasks = repo.list(TaskFilter {
        owner: Some("agent-2".to_string()),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(agent2_tasks.len(), 1);
    
    // Test filtering by state
    let created_tasks_filter = repo.list(TaskFilter {
        state: Some(TaskState::Created),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(created_tasks_filter.len(), 2);
    
    let in_progress_tasks = repo.list(TaskFilter {
        state: Some(TaskState::InProgress),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(in_progress_tasks.len(), 1);
    
    // Test combined filters
    let agent1_created = repo.list(TaskFilter {
        owner: Some("agent-1".to_string()),
        state: Some(TaskState::Created),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(agent1_created.len(), 1);
    
    // Test listing all
    let all_tasks = repo.list(TaskFilter::default()).await.unwrap();
    assert_eq!(all_tasks.len(), 3);
}

#[tokio::test]
async fn test_error_conditions() {
    let repo = create_test_repository().await;
    
    // Test empty field validation
    let invalid_task = NewTask {
        code: "".to_string(), // Empty code
        name: "Valid Name".to_string(),
        description: "Valid description".to_string(),
        owner_agent_name: "valid-agent".to_string(),
    };
    
    let result = repo.create(invalid_task).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::Validation(msg) => assert!(msg.contains("code")),
        other => panic!("Expected validation error, got: {:?}", other),
    }
    
    // Test duplicate code error
    let task1 = NewTask {
        code: "DUPLICATE".to_string(),
        name: "First Task".to_string(),
        description: "First task".to_string(),
        owner_agent_name: "agent-1".to_string(),
    };
    
    repo.create(task1.clone()).await.unwrap();
    
    let result = repo.create(task1).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::DuplicateCode(_) => {},
        other => panic!("Expected duplicate code error, got: {:?}", other),
    }
    
    // Test not found errors
    assert!(repo.update(99999, UpdateTask::default()).await.is_err());
    assert!(repo.set_state(99999, TaskState::InProgress).await.is_err());
    assert!(repo.assign(99999, "new-agent").await.is_err());
    assert!(repo.archive(99999).await.is_err());
    
    // Test invalid state transitions
    let task = repo.create(NewTask {
        code: "STATE-TEST".to_string(),
        name: "State Test".to_string(),
        description: "Test invalid transitions".to_string(),
        owner_agent_name: "test-agent".to_string(),
    }).await.unwrap();
    
    // Invalid transition: Created -> Archived (must go through Done first)
    let result = repo.set_state(task.id, TaskState::Archived).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::InvalidStateTransition(from, to) => {
            assert_eq!(from, TaskState::Created);
            assert_eq!(to, TaskState::Archived);
        },
        other => panic!("Expected invalid state transition error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_performance_requirements() {
    let repo = create_test_repository().await;
    
    // Create operation should complete in <100ms
    let start = Instant::now();
    let task = repo.create(NewTask {
        code: "PERF-001".to_string(),
        name: "Performance Test".to_string(),
        description: "Test performance requirements".to_string(),
        owner_agent_name: "perf-agent".to_string(),
    }).await.unwrap();
    let create_duration = start.elapsed();
    assert!(create_duration < Duration::from_millis(100), "Create took {:?}", create_duration);
    
    // Read operations should complete in <100ms
    let start = Instant::now();
    repo.get_by_id(task.id).await.unwrap();
    let read_duration = start.elapsed();
    assert!(read_duration < Duration::from_millis(100), "Read took {:?}", read_duration);
    
    // Update operations should complete in <100ms
    let start = Instant::now();
    repo.update(task.id, UpdateTask {
        name: Some("Updated Name".to_string()),
        ..Default::default()
    }).await.unwrap();
    let update_duration = start.elapsed();
    assert!(update_duration < Duration::from_millis(100), "Update took {:?}", update_duration);
    
    // List operations should complete in <100ms
    let start = Instant::now();
    repo.list(TaskFilter::default()).await.unwrap();
    let list_duration = start.elapsed();
    assert!(list_duration < Duration::from_millis(100), "List took {:?}", list_duration);
}

#[tokio::test]
async fn test_concurrent_operations() {
    let repo = create_test_repository().await;
    
    // Create multiple tasks concurrently
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let repo_clone = repo.clone();
        let handle = tokio::spawn(async move {
            let task = NewTask {
                code: format!("CONCURRENT-{:03}", i),
                name: format!("Concurrent Task {}", i),
                description: format!("Task created concurrently {}", i),
                owner_agent_name: "concurrent-agent".to_string(),
            };
            repo_clone.create(task).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await.unwrap());
    }
    
    // All operations should succeed
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(result.is_ok());
    }
    
    // Verify all tasks were created
    let all_tasks = repo.list(TaskFilter::default()).await.unwrap();
    assert_eq!(all_tasks.len(), 10);
}

#[tokio::test]
async fn test_repository_stats() {
    let repo = create_test_repository().await;
    
    // Create tasks with different states and owners
    let task1 = repo.create(NewTask {
        code: "STATS-001".to_string(),
        name: "Stats Test 1".to_string(),
        description: "First stats test task".to_string(),
        owner_agent_name: "agent-1".to_string(),
    }).await.unwrap();
    
    let task2 = repo.create(NewTask {
        code: "STATS-002".to_string(),
        name: "Stats Test 2".to_string(),
        description: "Second stats test task".to_string(),
        owner_agent_name: "agent-2".to_string(),
    }).await.unwrap();
    
    // Move tasks to different states
    repo.set_state(task1.id, TaskState::InProgress).await.unwrap();
    repo.set_state(task2.id, TaskState::InProgress).await.unwrap();
    repo.set_state(task2.id, TaskState::Done).await.unwrap();
    
    // Get stats
    let stats = repo.get_stats().await.unwrap();
    
    assert_eq!(stats.total_tasks, 2);
    assert_eq!(stats.tasks_by_state.get(&TaskState::InProgress), Some(&1));
    assert_eq!(stats.tasks_by_state.get(&TaskState::Done), Some(&1));
    assert_eq!(stats.tasks_by_owner.get("agent-1"), Some(&1));
    assert_eq!(stats.tasks_by_owner.get("agent-2"), Some(&1));
    assert!(stats.latest_created.is_some());
    assert!(stats.latest_completed.is_some());
}