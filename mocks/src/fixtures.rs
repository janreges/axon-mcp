//! Standard test fixtures for consistent testing
//! 
//! Provides pre-built test data including:
//! - Standard tasks in various states
//! - Edge case scenarios
//! - Bulk task generators

use task_core::{Task, TaskState, NewTask, UpdateTask};
use chrono::Utc;

/// Create a basic test task with sensible defaults
pub fn create_test_task() -> Task {
    Task {
        id: 1,
        code: "TEST-001".to_string(),
        name: "Test Task".to_string(),
        description: "A standard test task with default values".to_string(),
        owner_agent_name: "test-agent".to_string(),
        state: TaskState::Created,
        inserted_at: Utc::now(),
        done_at: None,
    }
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
    task.owner_agent_name = owner.to_string();
    task
}

/// Create multiple unique tasks
pub fn create_test_tasks(count: usize) -> Vec<Task> {
    (1..=count)
        .map(|i| Task {
            id: i as i32,
            code: format!("TEST-{i:03}"),
            name: format!("Test Task {i}"),
            description: format!("Test task number {i} for bulk testing"),
            owner_agent_name: format!("agent-{}", i % 3 + 1), // Distribute across 3 agents
            state: match i % 4 {
                0 => TaskState::Created,
                1 => TaskState::InProgress,
                2 => TaskState::Review,
                _ => TaskState::Done,
            },
            inserted_at: Utc::now(),
            done_at: if i % 4 == 3 { Some(Utc::now()) } else { None },
        })
        .collect()
}

/// Create one task in each possible state
pub fn create_tasks_in_all_states() -> Vec<Task> {
    vec![
        Task {
            id: 1,
            code: "CREATED-001".to_string(),
            name: "Created Task".to_string(),
            description: "Task in Created state".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        },
        Task {
            id: 2,
            code: "PROGRESS-001".to_string(),
            name: "InProgress Task".to_string(),
            description: "Task in InProgress state".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::InProgress,
            inserted_at: Utc::now(),
            done_at: None,
        },
        Task {
            id: 3,
            code: "BLOCKED-001".to_string(),
            name: "Blocked Task".to_string(),
            description: "Task in Blocked state".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Blocked,
            inserted_at: Utc::now(),
            done_at: None,
        },
        Task {
            id: 4,
            code: "REVIEW-001".to_string(),
            name: "Review Task".to_string(),
            description: "Task in Review state".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Review,
            inserted_at: Utc::now(),
            done_at: None,
        },
        Task {
            id: 5,
            code: "DONE-001".to_string(),
            name: "Done Task".to_string(),
            description: "Task in Done state".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Done,
            inserted_at: Utc::now(),
            done_at: Some(Utc::now()),
        },
        Task {
            id: 6,
            code: "ARCHIVED-001".to_string(),
            name: "Archived Task".to_string(),
            description: "Task in Archived state".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Archived,
            inserted_at: Utc::now(),
            done_at: Some(Utc::now()),
        },
    ]
}

/// Create a standard NewTask for testing creation
pub fn create_new_task() -> NewTask {
    NewTask {
        code: "NEW-001".to_string(),
        name: "New Test Task".to_string(),
        description: "A new task for testing creation".to_string(),
        owner_agent_name: "test-agent".to_string(),
    }
}

/// Create NewTask with specific code
pub fn create_new_task_with_code(code: &str) -> NewTask {
    let mut task = create_new_task();
    task.code = code.to_string();
    task
}

/// Create a standard UpdateTask for testing updates
pub fn create_update_task() -> UpdateTask {
    UpdateTask {
        name: Some("Updated Task Name".to_string()),
        description: Some("Updated task description".to_string()),
        owner_agent_name: Some("updated-agent".to_string()),
    }
}

/// Create UpdateTask with specific name
pub fn create_update_task_with_name(name: &str) -> UpdateTask {
    UpdateTask {
        name: Some(name.to_string()),
        description: None,
        owner_agent_name: None,
    }
}