//! Integration tests for the mocks crate
//!
//! Tests the mock implementations and utilities to ensure they work correctly
//! and provide the expected testing capabilities.

use mocks::*;
use task_core::{TaskError, TaskRepository, TaskState};

#[tokio::test]
async fn test_mock_repository_basic_operations() {
    let repo = MockTaskRepository::new();

    // Test creation
    let new_task = create_new_task();
    let task = repo.create(new_task).await.unwrap();

    assert_eq!(task.id, 1);
    assert_eq!(task.code, "NEW-001");
    assert_eq!(task.state, TaskState::Created);

    // Verify call tracking
    repo.assert_called("create");

    // Test retrieval
    let retrieved = repo.get_by_id(task.id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, task.id);

    repo.assert_called("get_by_id");
}

#[tokio::test]
async fn test_mock_repository_error_injection() {
    let repo = MockTaskRepository::new();

    // Inject error
    repo.inject_error(TaskError::NotFound("test error".to_string()));

    // Next operation should fail
    let result = repo.get_by_id(1).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TaskError::NotFound(_)));

    // Clear error and try again
    repo.clear_error();
    let result = repo.get_by_id(1).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mock_repository_state_transitions() {
    let repo = MockTaskRepository::new();

    // Create a task
    let new_task = create_new_task();
    let task = repo.create(new_task).await.unwrap();

    // Test valid state transition
    let updated_task = repo
        .set_state(task.id, TaskState::InProgress)
        .await
        .unwrap();
    assert_eq!(updated_task.state, TaskState::InProgress);

    // Test invalid state transition
    let result = repo.set_state(task.id, TaskState::Archived).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TaskError::InvalidStateTransition(_, _)
    ));
}

#[tokio::test]
async fn test_fixtures_create_tasks_in_all_states() {
    let tasks = create_tasks_in_all_states();

    assert_eq!(tasks.len(), 6);

    // Verify we have one task in each state
    let states: Vec<_> = tasks.iter().map(|t| t.state).collect();
    assert!(states.contains(&TaskState::Created));
    assert!(states.contains(&TaskState::InProgress));
    assert!(states.contains(&TaskState::Blocked));
    assert!(states.contains(&TaskState::Review));
    assert!(states.contains(&TaskState::Done));
    assert!(states.contains(&TaskState::Archived));

    // Verify done tasks have done_at timestamp
    let done_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.state == TaskState::Done)
        .collect();
    assert!(done_tasks[0].done_at.is_some());
}

#[tokio::test]
async fn test_builders_task_builder() {
    let task = TaskBuilder::new()
        .with_id(42)
        .with_code("BUILD-001")
        .with_name("Built Task")
        .with_state(TaskState::InProgress)
        .with_owner("builder-agent")
        .build();

    assert_eq!(task.id, 42);
    assert_eq!(task.code, "BUILD-001");
    assert_eq!(task.name, "Built Task");
    assert_eq!(task.state, TaskState::InProgress);
    assert_eq!(task.owner_agent_name.as_deref(), Some("builder-agent"));
}

#[tokio::test]
async fn test_assertions_task_equals() {
    let task1 = create_test_task();
    let mut task2 = task1.clone();

    // Should be equal
    assert_task_equals(&task1, &task2);

    // Change a field - should not be equal
    task2.name = "Different Name".to_string();

    let result = std::panic::catch_unwind(|| {
        assert_task_equals(&task1, &task2);
    });
    assert!(result.is_err());
}

#[tokio::test]
async fn test_assertions_state_transitions() {
    // Valid transitions should not panic
    assert_state_transition_valid(TaskState::Created, TaskState::InProgress);
    assert_state_transition_valid(TaskState::InProgress, TaskState::Done);

    // Invalid transitions should not panic (they test the negative case)
    assert_state_transition_invalid(TaskState::Created, TaskState::Done);
    assert_state_transition_invalid(TaskState::Archived, TaskState::InProgress);
}

#[tokio::test]
async fn test_generators_realistic_data() {
    let task = generate_random_task();

    // Verify generated data looks realistic
    assert!(task.id > 0);
    assert!(!task.code.is_empty());
    assert!(task.code.contains('-'));
    assert!(!task.name.is_empty());
    assert!(!task.description.is_empty());
    assert!(task.owner_agent_name.is_some());
}

#[tokio::test]
async fn test_mock_repository_concurrent_access() {
    use std::sync::Arc;
    use tokio::task::JoinSet;

    let repo = Arc::new(MockTaskRepository::new());
    let mut set = JoinSet::new();

    // Spawn multiple concurrent tasks
    for i in 0..10 {
        let repo_clone = repo.clone();
        set.spawn(async move {
            let new_task = NewTaskBuilder::new()
                .with_code(format!("CONCURRENT-{i:03}"))
                .with_name(format!("Concurrent Task {i}"))
                .build();

            repo_clone.create(new_task).await.unwrap()
        });
    }

    // Wait for all to complete
    let mut tasks = Vec::new();
    while let Some(result) = set.join_next().await {
        tasks.push(result.unwrap());
    }

    // Verify all tasks were created
    assert_eq!(tasks.len(), 10);

    // Verify unique IDs
    let mut ids: Vec<_> = tasks.iter().map(|t| t.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 10); // All IDs should be unique
}

#[tokio::test]
async fn test_mock_repository_stats() {
    let _repo = MockTaskRepository::new();

    // Create tasks in different states
    let tasks = create_tasks_in_all_states();
    let repo_with_tasks = MockTaskRepository::with_tasks(tasks);

    let stats = repo_with_tasks.get_stats().await.unwrap();

    assert_eq!(stats.total_tasks, 6);
    assert!(stats.tasks_by_state.contains_key(&TaskState::Created));
    assert!(stats.tasks_by_state.contains_key(&TaskState::Done));
    assert!(stats.latest_created.is_some());
}

#[tokio::test]
async fn test_contract_tests_with_mock() {
    let repo = MockTaskRepository::new();

    // Run the full contract test suite
    test_repository_contract(&repo).await;

    // Verify the mock was called multiple times
    let history = repo.call_history();
    assert!(
        !history.is_empty(),
        "Mock should have recorded method calls"
    );
    assert!(
        history.iter().any(|call| call.contains("create")),
        "Should have called create"
    );
    assert!(
        history.iter().any(|call| call.contains("get_by_id")),
        "Should have called get_by_id"
    );
}
