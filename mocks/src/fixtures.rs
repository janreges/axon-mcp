//! Standard test fixtures for consistent testing
//!
//! Provides pre-built test data including:
//! - Standard tasks in various states
//! - Edge case scenarios
//! - Bulk task generators

use chrono::Utc;
use task_core::{NewTask, Task, TaskState, UpdateTask};

/// Create a basic test task with sensible defaults
pub fn create_test_task() -> Task {
    Task::new(
        1,
        "TEST-001".to_string(),
        "Test Task".to_string(),
        "A standard test task with default values".to_string(),
        Some("test-agent".to_string()),
        TaskState::Created,
        Utc::now(),
        None,
    )
}

/// Create task with specific state
pub fn create_test_task_with_state(state: TaskState) -> Task {
    let mut task = create_test_task();
    task.state = state;

    // Set done_at if task is in Done state
    if state == TaskState::Done {
        task.done_at = Some(Utc::now());
    }

    task
}

/// Create task with specific owner
pub fn create_test_task_with_owner(owner: &str) -> Task {
    let mut task = create_test_task();
    task.owner_agent_name = Some(owner.to_string());
    task
}

/// Create multiple unique tasks
pub fn create_test_tasks(count: usize) -> Vec<Task> {
    (1..=count)
        .map(|i| {
            let state = match i % 4 {
                0 => TaskState::Created,
                1 => TaskState::InProgress,
                2 => TaskState::Review,
                _ => TaskState::Done,
            };
            let done_at = if i % 4 == 3 { Some(Utc::now()) } else { None };

            Task::new(
                i as i32,
                format!("TEST-{i:03}"),
                format!("Test Task {i}"),
                format!("Test task number {i} for bulk testing"),
                Some(format!("agent-{}", i % 3 + 1)), // Distribute across 3 agents
                state,
                Utc::now(),
                done_at,
            )
        })
        .collect()
}

/// Create one task in each possible state
pub fn create_tasks_in_all_states() -> Vec<Task> {
    let now = Utc::now();
    vec![
        Task::new(
            1,
            "CREATED-001".to_string(),
            "Created Task".to_string(),
            "Task in Created state".to_string(),
            Some("test-agent".to_string()),
            TaskState::Created,
            now,
            None,
        ),
        Task::new(
            2,
            "PROGRESS-001".to_string(),
            "InProgress Task".to_string(),
            "Task in InProgress state".to_string(),
            Some("test-agent".to_string()),
            TaskState::InProgress,
            now,
            None,
        ),
        Task::new(
            3,
            "BLOCKED-001".to_string(),
            "Blocked Task".to_string(),
            "Task in Blocked state".to_string(),
            Some("test-agent".to_string()),
            TaskState::Blocked,
            now,
            None,
        ),
        Task::new(
            4,
            "REVIEW-001".to_string(),
            "Review Task".to_string(),
            "Task in Review state".to_string(),
            Some("test-agent".to_string()),
            TaskState::Review,
            now,
            None,
        ),
        Task::new(
            5,
            "DONE-001".to_string(),
            "Done Task".to_string(),
            "Task in Done state".to_string(),
            Some("test-agent".to_string()),
            TaskState::Done,
            now,
            Some(now),
        ),
        Task::new(
            6,
            "ARCHIVED-001".to_string(),
            "Archived Task".to_string(),
            "Task in Archived state".to_string(),
            Some("test-agent".to_string()),
            TaskState::Archived,
            now,
            Some(now),
        ),
    ]
}

/// Create a standard NewTask for testing creation
pub fn create_new_task() -> NewTask {
    NewTask::new(
        "NEW-001".to_string(),
        "New Test Task".to_string(),
        "A new task for testing creation".to_string(),
        Some("test-agent".to_string()),
    )
}

/// Create NewTask with specific code
pub fn create_new_task_with_code(code: &str) -> NewTask {
    let mut task = create_new_task();
    task.code = code.to_string();
    task
}

/// Create a standard UpdateTask for testing updates
pub fn create_update_task() -> UpdateTask {
    UpdateTask::with_basic_fields(
        Some("Updated Task Name".to_string()),
        Some("Updated task description".to_string()),
        Some("updated-agent".to_string()),
    )
}

/// Create UpdateTask with specific name
pub fn create_update_task_with_name(name: &str) -> UpdateTask {
    UpdateTask::with_basic_fields(Some(name.to_string()), None, None)
}
