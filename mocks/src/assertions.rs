//! Custom assertion helpers for testing
//! 
//! Provides specialized assertions for:
//! - Task equality with clear error messages
//! - State transition validation
//! - Collection-based assertions

use task_core::{Task, TaskState};

/// Assert tasks are equal ignoring timestamps
pub fn assert_task_equals(actual: &Task, expected: &Task) {
    assert_eq!(actual.id, expected.id, "Task IDs don't match");
    assert_eq!(actual.code, expected.code, "Task codes don't match");
    assert_eq!(actual.name, expected.name, "Task names don't match");
    assert_eq!(actual.description, expected.description, "Task descriptions don't match");
    assert_eq!(actual.owner_agent_name, expected.owner_agent_name, "Task owners don't match");
    assert_eq!(actual.state, expected.state, "Task states don't match");
    // Note: timestamps are ignored in this assertion
}

/// Assert tasks are equal including exact timestamps
pub fn assert_task_equals_exact(actual: &Task, expected: &Task) {
    assert_eq!(actual, expected, "Tasks are not exactly equal");
}

/// Assert task matches partial criteria
pub fn assert_task_matches(task: &Task, matcher: &TaskMatcher) {
    if let Some(ref expected_id) = matcher.id {
        assert_eq!(task.id, *expected_id, "Task ID doesn't match expected");
    }
    if let Some(ref expected_code) = matcher.code {
        assert_eq!(task.code, *expected_code, "Task code doesn't match expected");
    }
    if let Some(ref expected_name) = matcher.name {
        assert_eq!(task.name, *expected_name, "Task name doesn't match expected");
    }
    if let Some(ref expected_owner) = matcher.owner_agent_name {
        assert_eq!(task.owner_agent_name, *expected_owner, "Task owner doesn't match expected");
    }
    if let Some(expected_state) = matcher.state {
        assert_eq!(task.state, expected_state, "Task state doesn't match expected");
    }
}

/// Assert state transition is valid according to business rules
pub fn assert_state_transition_valid(from: TaskState, to: TaskState) {
    let dummy_task = Task {
        id: 1,
        code: "TEST-001".to_string(),
        name: "Test".to_string(),
        description: "Test".to_string(),
        owner_agent_name: "test".to_string(),
        state: from,
        inserted_at: chrono::Utc::now(),
        done_at: None,
    };
    
    assert!(
        dummy_task.can_transition_to(to),
        "Expected transition from {from:?} to {to:?} to be valid, but it's not"
    );
}

/// Assert state transition is invalid according to business rules  
pub fn assert_state_transition_invalid(from: TaskState, to: TaskState) {
    let dummy_task = Task {
        id: 1,
        code: "TEST-001".to_string(),
        name: "Test".to_string(),
        description: "Test".to_string(),
        owner_agent_name: "test".to_string(),
        state: from,
        inserted_at: chrono::Utc::now(),
        done_at: None,
    };
    
    assert!(
        !dummy_task.can_transition_to(to),
        "Expected transition from {from:?} to {to:?} to be invalid, but it's valid"
    );
}

/// Assert task list contains task with specific code
pub fn assert_contains_task_with_code(tasks: &[Task], code: &str) {
    assert!(
        tasks.iter().any(|t| t.code == code),
        "Expected to find task with code '{}' in task list, but it wasn't found. Available codes: {:?}",
        code,
        tasks.iter().map(|t| &t.code).collect::<Vec<_>>()
    );
}

/// Assert tasks are sorted by insertion date (most recent first)
pub fn assert_tasks_sorted_by_date(tasks: &[Task]) {
    for window in tasks.windows(2) {
        assert!(
            window[0].inserted_at >= window[1].inserted_at,
            "Tasks are not sorted by insertion date (most recent first). Task '{}' ({}) comes before '{}' ({})",
            window[0].code,
            window[0].inserted_at,
            window[1].code,
            window[1].inserted_at
        );
    }
}

/// Flexible task matcher for partial assertions
#[derive(Debug, Default)]
pub struct TaskMatcher {
    pub id: Option<i32>,
    pub code: Option<String>,
    pub name: Option<String>,
    pub owner_agent_name: Option<String>,
    pub state: Option<TaskState>,
}

impl TaskMatcher {
    /// Create a new empty matcher
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Match tasks with specific ID
    pub fn with_id(mut self, id: i32) -> Self {
        self.id = Some(id);
        self
    }
    
    /// Match tasks with specific code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
    
    /// Match tasks with specific name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    /// Match tasks with specific owner
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner_agent_name = Some(owner.into());
        self
    }
    
    /// Match tasks with specific state
    pub fn with_state(mut self, state: TaskState) -> Self {
        self.state = Some(state);
        self
    }
}