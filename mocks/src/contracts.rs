//! Contract test helpers for validating trait implementations
//!
//! Provides standardized tests that any implementation of core traits
//! should pass, ensuring consistent behavior across different implementations.

use crate::{create_new_task, NewTaskBuilder, TaskFilterBuilder, UpdateTaskBuilder};
use task_core::{TaskError, TaskRepository, TaskState};

/// Test any TaskRepository implementation with comprehensive contract tests
///
/// This function runs a suite of tests that any TaskRepository implementation
/// should pass to be considered compliant with the expected contract.
pub async fn test_repository_contract<R: TaskRepository>(repo: &R) {
    test_create_contract(repo).await;
    test_update_contract(repo).await;
    test_state_contract(repo).await;
    test_get_contract(repo).await;
    test_list_contract(repo).await;
    test_assign_contract(repo).await;
    test_archive_contract(repo).await;
    test_health_check_contract(repo).await;
    test_stats_contract(repo).await;
}

/// Test task creation contract
pub async fn test_create_contract<R: TaskRepository>(repo: &R) {
    // Test successful creation
    let new_task = create_new_task();
    let task = repo
        .create(new_task.clone())
        .await
        .expect("Create should succeed");

    assert!(task.id > 0, "Created task should have positive ID");
    assert_eq!(
        task.code, new_task.code,
        "Created task should preserve code"
    );
    assert_eq!(
        task.name, new_task.name,
        "Created task should preserve name"
    );
    assert_eq!(
        task.state,
        TaskState::Created,
        "New task should start in Created state"
    );
    assert!(
        task.done_at.is_none(),
        "New task should not have done_at timestamp"
    );

    // Test duplicate code rejection
    let duplicate_result = repo.create(new_task).await;
    assert!(
        duplicate_result.is_err(),
        "Should reject duplicate task codes"
    );
    match duplicate_result.unwrap_err() {
        TaskError::DuplicateCode(_) => {} // Expected
        other => panic!("Expected DuplicateCode error, got: {other:?}"),
    }
}

/// Test task update contract
pub async fn test_update_contract<R: TaskRepository>(repo: &R) {
    // Create a task first
    let new_task = NewTaskBuilder::new().with_code("UPDATE-TEST").build();
    let task = repo.create(new_task).await.expect("Create should succeed");

    // Test successful update
    let update = UpdateTaskBuilder::new()
        .with_name("Updated Name")
        .with_description("Updated Description")
        .build();

    let updated_task = repo
        .update(task.id, update)
        .await
        .expect("Update should succeed");
    assert_eq!(updated_task.name, "Updated Name");
    assert_eq!(updated_task.description, "Updated Description");
    assert_eq!(updated_task.id, task.id, "ID should remain unchanged");
    assert_eq!(updated_task.code, task.code, "Code should remain unchanged");

    // Test update non-existent task
    let update_result = repo.update(99999, UpdateTaskBuilder::new().build()).await;
    assert!(
        update_result.is_err(),
        "Should fail to update non-existent task"
    );
    match update_result.unwrap_err() {
        TaskError::NotFound(_) => {} // Expected
        other => panic!("Expected NotFound error, got: {other:?}"),
    }
}

/// Test state transition contract
pub async fn test_state_contract<R: TaskRepository>(repo: &R) {
    // Create a task first
    let new_task = NewTaskBuilder::new().with_code("STATE-TEST").build();
    let task = repo.create(new_task).await.expect("Create should succeed");

    // Test valid state transition
    let updated_task = repo
        .set_state(task.id, TaskState::InProgress)
        .await
        .expect("Valid state transition should succeed");
    assert_eq!(updated_task.state, TaskState::InProgress);

    // Test completion sets done_at
    let done_task = repo
        .set_state(task.id, TaskState::Done)
        .await
        .expect("Transition to Done should succeed");
    assert_eq!(done_task.state, TaskState::Done);
    assert!(
        done_task.done_at.is_some(),
        "Done task should have done_at timestamp"
    );

    // Test invalid state transition (Done -> InProgress is not allowed)
    let invalid_result = repo.set_state(task.id, TaskState::InProgress).await;
    assert!(
        invalid_result.is_err(),
        "Should reject invalid state transition"
    );
    match invalid_result.unwrap_err() {
        TaskError::InvalidStateTransition(_, _) => {} // Expected
        other => panic!("Expected InvalidStateTransition error, got: {other:?}"),
    }

    // Test state change on non-existent task
    let not_found_result = repo.set_state(99999, TaskState::InProgress).await;
    assert!(
        not_found_result.is_err(),
        "Should fail for non-existent task"
    );
}

/// Test get operations contract
pub async fn test_get_contract<R: TaskRepository>(repo: &R) {
    // Create a task first
    let new_task = NewTaskBuilder::new().with_code("GET-TEST").build();
    let task = repo.create(new_task).await.expect("Create should succeed");

    // Test get by ID
    let retrieved_by_id = repo
        .get_by_id(task.id)
        .await
        .expect("Get by ID should succeed")
        .expect("Task should exist");
    assert_eq!(retrieved_by_id.id, task.id);
    assert_eq!(retrieved_by_id.code, task.code);

    // Test get by code
    let retrieved_by_code = repo
        .get_by_code(&task.code)
        .await
        .expect("Get by code should succeed")
        .expect("Task should exist");
    assert_eq!(retrieved_by_code.id, task.id);
    assert_eq!(retrieved_by_code.code, task.code);

    // Test get non-existent by ID
    let not_found_by_id = repo
        .get_by_id(99999)
        .await
        .expect("Get by ID should not error for non-existent ID");
    assert!(
        not_found_by_id.is_none(),
        "Should return None for non-existent ID"
    );

    // Test get non-existent by code
    let not_found_by_code = repo
        .get_by_code("NON-EXISTENT")
        .await
        .expect("Get by code should not error for non-existent code");
    assert!(
        not_found_by_code.is_none(),
        "Should return None for non-existent code"
    );
}

/// Test list operations contract
pub async fn test_list_contract<R: TaskRepository>(repo: &R) {
    // Create multiple tasks with different properties
    let tasks = vec![
        NewTaskBuilder::new()
            .with_code("LIST-1")
            .with_owner_agent_name("agent-1")
            .build(),
        NewTaskBuilder::new()
            .with_code("LIST-2")
            .with_owner_agent_name("agent-2")
            .build(),
        NewTaskBuilder::new()
            .with_code("LIST-3")
            .with_owner_agent_name("agent-1")
            .build(),
    ];

    let mut created_tasks = Vec::new();
    for new_task in tasks {
        let task = repo.create(new_task).await.expect("Create should succeed");
        created_tasks.push(task);
    }

    // Set different states
    repo.set_state(created_tasks[1].id, TaskState::InProgress)
        .await
        .expect("State change should succeed");

    // Test list all
    let all_tasks = repo
        .list(TaskFilterBuilder::new().build())
        .await
        .expect("List all should succeed");
    assert!(
        all_tasks.len() >= 3,
        "Should contain at least our created tasks"
    );

    // Test filter by owner
    let agent1_tasks = repo
        .list(TaskFilterBuilder::new().with_owner("agent-1").build())
        .await
        .expect("Filter by owner should succeed");
    assert!(
        agent1_tasks
            .iter()
            .all(|t| t.owner_agent_name.as_deref() == Some("agent-1")),
        "All returned tasks should be owned by agent-1"
    );

    // Test filter by state
    let in_progress_tasks = repo
        .list(
            TaskFilterBuilder::new()
                .with_state(TaskState::InProgress)
                .build(),
        )
        .await
        .expect("Filter by state should succeed");
    assert!(
        in_progress_tasks
            .iter()
            .all(|t| t.state == TaskState::InProgress),
        "All returned tasks should be in InProgress state"
    );
}

/// Test task assignment contract
pub async fn test_assign_contract<R: TaskRepository>(repo: &R) {
    // Create a task first
    let new_task = NewTaskBuilder::new()
        .with_code("ASSIGN-TEST")
        .with_owner_agent_name("original-owner")
        .build();
    let task = repo.create(new_task).await.expect("Create should succeed");

    // Test successful assignment
    let assigned_task = repo
        .assign(task.id, "new-owner")
        .await
        .expect("Assignment should succeed");
    assert_eq!(assigned_task.owner_agent_name.as_deref(), Some("new-owner"));
    assert_eq!(assigned_task.id, task.id, "ID should remain unchanged");

    // Test assignment to empty owner (should fail)
    let empty_owner_result = repo.assign(task.id, "").await;
    assert!(
        empty_owner_result.is_err(),
        "Should reject empty owner name"
    );

    // Test assignment of non-existent task
    let not_found_result = repo.assign(99999, "some-owner").await;
    assert!(
        not_found_result.is_err(),
        "Should fail for non-existent task"
    );
}

/// Test task archival contract
pub async fn test_archive_contract<R: TaskRepository>(repo: &R) {
    // Create and complete a task first
    let new_task = NewTaskBuilder::new().with_code("ARCHIVE-TEST").build();
    let task = repo.create(new_task).await.expect("Create should succeed");

    // Follow valid state transitions: Created -> InProgress -> Done
    let in_progress_task = repo
        .set_state(task.id, TaskState::InProgress)
        .await
        .expect("Set to InProgress should succeed");
    let done_task = repo
        .set_state(in_progress_task.id, TaskState::Done)
        .await
        .expect("Set to Done should succeed");

    // Test successful archival
    let archived_task = repo
        .archive(done_task.id)
        .await
        .expect("Archive should succeed");
    assert_eq!(archived_task.state, TaskState::Archived);
    assert_eq!(archived_task.id, task.id, "ID should remain unchanged");

    // Test archival of task in invalid state
    let new_task2 = NewTaskBuilder::new().with_code("ARCHIVE-TEST-2").build();
    let task2 = repo.create(new_task2).await.expect("Create should succeed");

    let invalid_archive_result = repo.archive(task2.id).await;
    assert!(
        invalid_archive_result.is_err(),
        "Should reject archival of non-completed task"
    );

    // Test archival of non-existent task
    let not_found_result = repo.archive(99999).await;
    assert!(
        not_found_result.is_err(),
        "Should fail for non-existent task"
    );
}

/// Test health check contract
pub async fn test_health_check_contract<R: TaskRepository>(repo: &R) {
    // Health check should succeed for a working repository
    let health_result = repo.health_check().await;
    assert!(
        health_result.is_ok(),
        "Health check should succeed for working repository"
    );
}

/// Test statistics contract
pub async fn test_stats_contract<R: TaskRepository>(repo: &R) {
    // Create some tasks for statistics
    let new_task = NewTaskBuilder::new().with_code("STATS-TEST").build();
    let _task = repo.create(new_task).await.expect("Create should succeed");

    // Get stats
    let stats = repo.get_stats().await.expect("Get stats should succeed");

    // Verify basic stats structure
    assert!(stats.total_tasks > 0, "Should report at least one task");
    assert!(
        !stats.tasks_by_state.is_empty(),
        "Should have state breakdown"
    );
    assert!(
        !stats.tasks_by_owner.is_empty(),
        "Should have owner breakdown"
    );
    assert!(
        stats.latest_created.is_some(),
        "Should have latest creation timestamp"
    );
}
