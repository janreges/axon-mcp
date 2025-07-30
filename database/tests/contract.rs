use database::{TaskRepository, NewTask, UpdateTask, TaskFilter, TaskState, TaskError};
use std::sync::Arc;

/// Contract tests that all TaskRepository implementations must pass
/// 
/// These tests verify that implementations correctly handle all operations
/// defined in the TaskRepository trait, including edge cases and error conditions.

#[allow(dead_code)]
pub async fn test_repository_contract<R: TaskRepository + Clone + Send + Sync + 'static>(repo: Arc<R>) {
    test_health_check(repo.clone()).await;
    test_create_task_contract(repo.clone()).await;
    test_get_by_id_contract(repo.clone()).await;
    test_get_by_code_contract(repo.clone()).await;
    test_update_task_contract(repo.clone()).await;
    test_state_transitions_contract(repo.clone()).await;
    test_task_assignment_contract(repo.clone()).await;
    test_task_archiving_contract(repo.clone()).await;
    test_task_listing_contract(repo.clone()).await;
    test_validation_errors_contract(repo.clone()).await;
    test_not_found_errors_contract(repo.clone()).await;
    test_stats_contract(repo.clone()).await;
}

async fn test_health_check<R: TaskRepository>(repo: Arc<R>) {
    assert!(repo.health_check().await.is_ok(), "Health check should pass for healthy repository");
}

async fn test_create_task_contract<R: TaskRepository>(repo: Arc<R>) {
    let new_task = NewTask::new("CONTRACT-CREATE".to_string(), "Contract Create Test".to_string(), "Test task creation contract".to_string(), Some("contract-agent".to_string()),  );
    
    let created = repo.create(new_task).await.unwrap();
    
    // Verify task properties
    assert_eq!(created.code, "CONTRACT-CREATE");
    assert_eq!(created.name, "Contract Create Test");
    assert_eq!(created.description, "Test task creation contract");
    assert_eq!(created.owner_agent_name.as_deref(), Some("contract-agent"));
    assert_eq!(created.state, TaskState::Created);
    assert!(created.id > 0);
    assert!(created.done_at.is_none());
    assert!(created.inserted_at <= chrono::Utc::now());
}

async fn test_get_by_id_contract<R: TaskRepository>(repo: Arc<R>) {
    // Create a task to retrieve
    let new_task = NewTask::new("CONTRACT-GET-ID".to_string(), "Contract Get ID Test".to_string(), "Test get by ID contract".to_string(), Some("contract-agent".to_string()),  );
    
    let created = repo.create(new_task).await.unwrap();
    
    // Test successful retrieval
    let retrieved = repo.get_by_id(created.id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, created.id);
    
    // Test non-existent ID returns None (not error)
    let not_found = repo.get_by_id(99999).await.unwrap();
    assert!(not_found.is_none());
}

async fn test_get_by_code_contract<R: TaskRepository>(repo: Arc<R>) {
    // Create a task to retrieve
    let new_task = NewTask::new("CONTRACT-GET-CODE".to_string(), "Contract Get Code Test".to_string(), "Test get by code contract".to_string(), Some("contract-agent".to_string()),  );
    
    let _created = repo.create(new_task).await.unwrap();
    
    // Test successful retrieval
    let retrieved = repo.get_by_code("CONTRACT-GET-CODE").await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().code, "CONTRACT-GET-CODE");
    
    // Test non-existent code returns None (not error)
    let not_found = repo.get_by_code("NON-EXISTENT-CODE").await.unwrap();
    assert!(not_found.is_none());
}

async fn test_update_task_contract<R: TaskRepository>(repo: Arc<R>) {
    // Create a task to update
    let new_task = NewTask::new("CONTRACT-UPDATE".to_string(), "Original Name".to_string(), "Original description".to_string(), Some("original-agent".to_string()),  );
    
    let created = repo.create(new_task).await.unwrap();
    
    // Test full update
    let updates = UpdateTask {
        name: Some("Updated Name".to_string()),
        description: Some("Updated description".to_string()),
        owner_agent_name: Some("updated-agent".to_string()),
        required_capabilities: None,
        priority_score: None,
        confidence_threshold: None,
        estimated_effort: None,
        parent_task_id: None,
        workflow_definition_id: None,
        workflow_cursor: None,
    };
    
    let updated = repo.update(created.id, updates).await.unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.description, "Updated description");
    assert_eq!(updated.owner_agent_name.as_deref(), Some("updated-agent"));
    assert_eq!(updated.code, "CONTRACT-UPDATE"); // Code should not change
    assert_eq!(updated.id, created.id); // ID should not change
    
    // Test partial update
    let partial_updates = UpdateTask {
        name: Some("Partially Updated".to_string()),
        description: None,
        owner_agent_name: None,
        required_capabilities: None,
        priority_score: None,
        confidence_threshold: None,
        estimated_effort: None,
        parent_task_id: None,
        workflow_definition_id: None,
        workflow_cursor: None,
    };
    
    let partially_updated = repo.update(created.id, partial_updates).await.unwrap();
    assert_eq!(partially_updated.name, "Partially Updated");
    assert_eq!(partially_updated.description, "Updated description"); // Should remain unchanged
    assert_eq!(partially_updated.owner_agent_name.as_deref(), Some("updated-agent")); // Should remain unchanged
    
    // Test empty update (no changes)
    let no_updates = UpdateTask::default();
    let unchanged = repo.update(created.id, no_updates).await.unwrap();
    assert_eq!(unchanged.name, partially_updated.name);
    assert_eq!(unchanged.description, partially_updated.description);
    assert_eq!(unchanged.owner_agent_name, partially_updated.owner_agent_name);
}

async fn test_state_transitions_contract<R: TaskRepository>(repo: Arc<R>) {
    // Create a task for state transition testing
    let new_task = NewTask::new("CONTRACT-STATES".to_string(), "Contract States Test".to_string(), "Test state transitions contract".to_string(), Some("contract-agent".to_string()),  );
    
    let mut task = repo.create(new_task).await.unwrap();
    assert_eq!(task.state, TaskState::Created);
    
    // Valid transitions from Created
    task = repo.set_state(task.id, TaskState::InProgress).await.unwrap();
    assert_eq!(task.state, TaskState::InProgress);
    assert!(task.done_at.is_none());
    
    // Valid transitions from InProgress
    task = repo.set_state(task.id, TaskState::Blocked).await.unwrap();
    assert_eq!(task.state, TaskState::Blocked);
    
    task = repo.set_state(task.id, TaskState::InProgress).await.unwrap();
    assert_eq!(task.state, TaskState::InProgress);
    
    task = repo.set_state(task.id, TaskState::Review).await.unwrap();
    assert_eq!(task.state, TaskState::Review);
    
    task = repo.set_state(task.id, TaskState::Done).await.unwrap();
    assert_eq!(task.state, TaskState::Done);
    assert!(task.done_at.is_some()); // done_at should be set when moving to Done
    
    // Test invalid transitions
    let result = repo.set_state(task.id, TaskState::InProgress).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::InvalidStateTransition(from, to) => {
            assert_eq!(from, TaskState::Done);
            assert_eq!(to, TaskState::InProgress);
        },
        other => panic!("Expected InvalidStateTransition error, got: {:?}", other),
    }
    
    // Test same state transition (should fail)
    let result = repo.set_state(task.id, TaskState::Done).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::InvalidStateTransition(from, to) => {
            assert_eq!(from, TaskState::Done);
            assert_eq!(to, TaskState::Done);
        },
        other => panic!("Expected InvalidStateTransition error, got: {:?}", other),
    }
}

async fn test_task_assignment_contract<R: TaskRepository>(repo: Arc<R>) {
    // Create a task for assignment testing
    let new_task = NewTask::new("CONTRACT-ASSIGN".to_string(), "Contract Assign Test".to_string(), "Test task assignment contract".to_string(), Some("original-agent".to_string()),  );
    
    let task = repo.create(new_task).await.unwrap();
    
    // Test successful assignment
    let assigned = repo.assign(task.id, "new-agent").await.unwrap();
    assert_eq!(assigned.owner_agent_name.as_deref(), Some("new-agent"));
    assert_eq!(assigned.id, task.id); // Other fields should remain unchanged
    assert_eq!(assigned.code, task.code);
    assert_eq!(assigned.name, task.name);
    
    // Verify assignment persisted
    let retrieved = repo.get_by_id(task.id).await.unwrap().unwrap();
    assert_eq!(retrieved.owner_agent_name.as_deref(), Some("new-agent"));
}

async fn test_task_archiving_contract<R: TaskRepository>(repo: Arc<R>) {
    // Create and complete a task for archiving
    let new_task = NewTask::new("CONTRACT-ARCHIVE".to_string(), "Contract Archive Test".to_string(), "Test task archiving contract".to_string(), Some("contract-agent".to_string()),  );
    
    let mut task = repo.create(new_task).await.unwrap();
    
    // Move to Done state (required for archiving)
    task = repo.set_state(task.id, TaskState::InProgress).await.unwrap();
    task = repo.set_state(task.id, TaskState::Done).await.unwrap();
    
    // Test successful archiving
    let archived = repo.archive(task.id).await.unwrap();
    assert_eq!(archived.state, TaskState::Archived);
    assert_eq!(archived.id, task.id);
    
    // Verify archived task can still be retrieved
    let retrieved = repo.get_by_id(task.id).await.unwrap().unwrap();
    assert_eq!(retrieved.state, TaskState::Archived);
    
    // Test archiving non-Done task should fail
    let new_task2 = NewTask::new("CONTRACT-ARCHIVE-2".to_string(), "Archive Test 2".to_string(), "Test invalid archiving".to_string(), Some("contract-agent".to_string()),  );
    
    let task2 = repo.create(new_task2).await.unwrap();
    let result = repo.archive(task2.id).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::InvalidStateTransition(from, to) => {
            assert_eq!(from, TaskState::Created);
            assert_eq!(to, TaskState::Archived);
        },
        other => panic!("Expected InvalidStateTransition error, got: {:?}", other),
    }
}

async fn test_task_listing_contract<R: TaskRepository>(repo: Arc<R>) {
    // Create tasks with different properties for filtering
    let tasks = vec![
        NewTask::new("CONTRACT-LIST-1".to_string(), "List Test 1".to_string(), "First list test task".to_string(), Some("agent-1".to_string())),
        NewTask::new("CONTRACT-LIST-2".to_string(), "List Test 2".to_string(), "Second list test task".to_string(), Some("agent-1".to_string())),
        NewTask::new("CONTRACT-LIST-3".to_string(), "List Test 3".to_string(), "Third list test task".to_string(), Some("agent-2".to_string())),
    ];
    
    let mut created_tasks = Vec::new();
    for task in tasks {
        created_tasks.push(repo.create(task).await.unwrap());
    }
    
    // Move one task to different state
    repo.set_state(created_tasks[0].id, TaskState::InProgress).await.unwrap();
    
    // Test listing all tasks
    let all_tasks = repo.list(TaskFilter::default()).await.unwrap();
    assert!(all_tasks.len() >= 3); // May include tasks from other tests
    
    // Test filtering by owner
    let agent1_tasks = repo.list(TaskFilter {
        owner: Some("agent-1".to_string()),
        ..Default::default()
    }).await.unwrap();
    
    let agent1_count = agent1_tasks.iter()
        .filter(|t| t.owner_agent_name.as_deref() == Some("agent-1"))
        .count();
    assert!(agent1_count >= 2);
    
    // Test filtering by state
    let created_tasks_filter = repo.list(TaskFilter {
        state: Some(TaskState::Created),
        ..Default::default()
    }).await.unwrap();
    
    let created_count = created_tasks_filter.iter()
        .filter(|t| t.state == TaskState::Created)
        .count();
    assert!(created_count >= 2);
    
    // Test combined filters
    let agent1_created = repo.list(TaskFilter {
        owner: Some("agent-1".to_string()),
        state: Some(TaskState::Created),
        ..Default::default()
    }).await.unwrap();
    
    let combined_count = agent1_created.iter()
        .filter(|t| t.owner_agent_name.as_deref() == Some("agent-1") && t.state == TaskState::Created)
        .count();
    assert!(combined_count >= 1);
}

async fn test_validation_errors_contract<R: TaskRepository>(repo: Arc<R>) {
    // Test empty field validation for create
    let invalid_tasks = vec![
        NewTask::new("".to_string(), // Empty code
            "Valid Name".to_string(), "Valid description".to_string(), Some("valid-agent".to_string())),
        NewTask::new("VALID-CODE".to_string(), "".to_string(), // Empty name
            "Valid description".to_string(), Some("valid-agent".to_string())),
        NewTask::new("VALID-CODE".to_string(), "Valid Name".to_string(), "".to_string(), // Empty description
            Some("valid-agent".to_string())),
        NewTask::new("VALID-CODE".to_string(), "Valid Name".to_string(), "Valid description".to_string(), Some("".to_string())), // Empty owner
    ];
    
    for invalid_task in invalid_tasks {
        let result = repo.create(invalid_task).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            TaskError::Validation(_) => {}, // Expected
            other => panic!("Expected Validation error, got: {:?}", other),
        }
    }
    
    // Test duplicate code error
    let task1 = NewTask::new("CONTRACT-DUPLICATE".to_string(), "First Task".to_string(), "First task with this code".to_string(), Some("agent-1".to_string()),  );
    
    repo.create(task1.clone()).await.unwrap();
    
    let result = repo.create(task1).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::DuplicateCode(_) => {}, // Expected
        other => panic!("Expected DuplicateCode error, got: {:?}", other),
    }
    
    // Test empty field validation for assignment
    let valid_task = NewTask::new("CONTRACT-ASSIGN-VALID".to_string(), "Valid Task".to_string(), "Valid task for assignment test".to_string(), Some("original-agent".to_string()),  );
    
    let task = repo.create(valid_task).await.unwrap();
    
    let result = repo.assign(task.id, "").await; // Empty new owner
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::Validation(_) => {}, // Expected
        other => panic!("Expected Validation error, got: {:?}", other),
    }
}

async fn test_not_found_errors_contract<R: TaskRepository>(repo: Arc<R>) {
    let non_existent_id = 99999;
    
    // Test update on non-existent task
    let result = repo.update(non_existent_id, UpdateTask::default()).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::NotFound(_) => {}, // Expected
        other => panic!("Expected NotFound error, got: {:?}", other),
    }
    
    // Test set_state on non-existent task
    let result = repo.set_state(non_existent_id, TaskState::InProgress).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::NotFound(_) => {}, // Expected
        other => panic!("Expected NotFound error, got: {:?}", other),
    }
    
    // Test assign on non-existent task
    let result = repo.assign(non_existent_id, "new-agent").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::NotFound(_) => {}, // Expected
        other => panic!("Expected NotFound error, got: {:?}", other),
    }
    
    // Test archive on non-existent task
    let result = repo.archive(non_existent_id).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TaskError::NotFound(_) => {}, // Expected
        other => panic!("Expected NotFound error, got: {:?}", other),
    }
}

async fn test_stats_contract<R: TaskRepository>(repo: Arc<R>) {
    // Create tasks for stats testing
    let task1 = repo.create(NewTask::new("CONTRACT-STATS-1".to_string(), "Stats Test 1".to_string(), "First stats test task".to_string(), Some("stats-agent-1".to_string()))).await.unwrap();
    
    let task2 = repo.create(NewTask::new("CONTRACT-STATS-2".to_string(), "Stats Test 2".to_string(), "Second stats test task".to_string(), Some("stats-agent-2".to_string()))).await.unwrap();
    
    // Move tasks to different states
    repo.set_state(task1.id, TaskState::InProgress).await.unwrap();
    repo.set_state(task2.id, TaskState::InProgress).await.unwrap();
    repo.set_state(task2.id, TaskState::Done).await.unwrap();
    
    // Get stats
    let stats = repo.get_stats().await.unwrap();
    
    // Verify stats structure (exact counts may vary due to other tests)
    assert!(stats.total_tasks >= 2);
    assert!(stats.tasks_by_state.contains_key(&TaskState::InProgress));
    assert!(stats.tasks_by_state.contains_key(&TaskState::Done));
    assert!(stats.tasks_by_owner.contains_key("stats-agent-1"));
    assert!(stats.tasks_by_owner.contains_key("stats-agent-2"));
    assert!(stats.latest_created.is_some());
    assert!(stats.latest_completed.is_some());
}

// Test the SQLite implementation against the contract
#[tokio::test]
async fn test_sqlite_repository_contract() {
    use database::SqliteTaskRepository;
    
    // Use a unique database name to avoid conflicts with other tests
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let thread_id = std::thread::current().id();
    let db_name = format!(":memory:contract_{}_{:?}", timestamp, thread_id);
    
    let repo = SqliteTaskRepository::new(&db_name).await.unwrap();
    repo.migrate().await.unwrap();
    
    test_repository_contract(Arc::new(repo)).await;
}