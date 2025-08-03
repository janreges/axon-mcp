//! Random test data generators using the fake crate
//!
//! Provides realistic random data including:
//! - Task codes with proper formatting
//! - Agent names from a realistic pool
//! - Task names and descriptions
//! - Property-based testing strategies

use chrono::Utc;
use fake::faker::lorem::en::{Paragraph, Sentence};
use fake::Fake;
use proptest::prelude::*;
use rand::Rng;
use task_core::{Task, TaskFilter, TaskState};

/// Generate a realistic task code (e.g., "PROJ-123", "BUG-456")
pub fn generate_task_code() -> String {
    let prefixes = ["PROJ", "BUG", "FEAT", "DOCS", "TEST", "REFAC"];
    let prefix = prefixes[rand::thread_rng().gen_range(0..prefixes.len())];
    let number: u32 = (1..9999).fake();
    format!("{prefix}-{number:03}")
}

/// Generate a realistic agent name
pub fn generate_agent_name() -> String {
    let agents = [
        "rust-architect",
        "database-engineer",
        "protocol-specialist",
        "integration-lead",
        "testing-expert",
        "documentation-specialist",
        "project-finalizer",
        "security-auditor",
        "performance-optimizer",
    ];
    agents[rand::thread_rng().gen_range(0..agents.len())].to_string()
}

/// Generate a realistic task name
pub fn generate_task_name() -> String {
    Sentence(3..8).fake()
}

/// Generate a realistic task description
pub fn generate_task_description() -> String {
    Paragraph(2..5).fake()
}

/// Generate a random task with realistic data
pub fn generate_random_task() -> Task {
    let id: u32 = (1..99999).fake();
    Task::new(
        id as i32,
        generate_task_code(),
        generate_task_name(),
        generate_task_description(),
        Some(generate_agent_name()),
        generate_random_task_state(),
        Utc::now(),
        None,
    )
}

/// Generate a random task state
pub fn generate_random_task_state() -> TaskState {
    let states = [
        TaskState::Created,
        TaskState::InProgress,
        TaskState::Blocked,
        TaskState::Review,
        TaskState::Done,
        TaskState::Archived,
    ];
    states[rand::thread_rng().gen_range(0..states.len())]
}

/// Configurable task generator
pub struct TaskGenerator {
    pub code_prefix: String,
    pub agent_pool: Vec<String>,
}

impl Default for TaskGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskGenerator {
    /// Create new generator with default settings
    pub fn new() -> Self {
        Self {
            code_prefix: "TEST".to_string(),
            agent_pool: vec![
                "agent-1".to_string(),
                "agent-2".to_string(),
                "agent-3".to_string(),
            ],
        }
    }

    /// Generate task with this generator's settings
    pub fn generate(&self) -> Task {
        let id: u32 = (1..99999).fake();
        let number: u32 = (1..9999).fake();
        let agent = &self.agent_pool[rand::thread_rng().gen_range(0..self.agent_pool.len())];

        Task::new(
            id as i32,
            format!("{}-{number:03}", self.code_prefix),
            generate_task_name(),
            generate_task_description(),
            Some(agent.clone()),
            generate_random_task_state(),
            Utc::now(),
            None,
        )
    }
}

/// Proptest strategy for generating valid task codes
pub fn task_code_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec("[A-Z]{3,8}-[0-9]{1,4}", 1..1).prop_map(|v| v[0].clone())
}

/// Proptest strategy for generating valid task states
pub fn task_state_strategy() -> impl Strategy<Value = TaskState> {
    prop_oneof![
        Just(TaskState::Created),
        Just(TaskState::InProgress),
        Just(TaskState::Blocked),
        Just(TaskState::Review),
        Just(TaskState::Done),
        Just(TaskState::Archived),
    ]
}

/// Proptest strategy for generating complete tasks
pub fn task_strategy() -> impl Strategy<Value = Task> {
    (
        1i32..99999,
        task_code_strategy(),
        "[A-Za-z ]{5,50}",
        "[A-Za-z0-9 .,!?]{10,200}",
        "[a-z-]{5,20}",
        task_state_strategy(),
    )
        .prop_map(|(id, code, name, description, owner, state)| {
            let done_at = if state == TaskState::Done || state == TaskState::Archived {
                Some(Utc::now())
            } else {
                None
            };
            Task::new(
                id,
                code,
                name,
                description,
                Some(owner),
                state,
                Utc::now(),
                done_at,
            )
        })
}

/// Proptest strategy for generating task filters
pub fn task_filter_strategy() -> impl Strategy<Value = TaskFilter> {
    (
        proptest::option::of("[a-z-]{5,20}"),
        proptest::option::of(task_state_strategy()),
    )
        .prop_map(|(owner, state)| TaskFilter {
            owner,
            state,
            date_from: None,
            date_to: None,
            completed_after: None,
            completed_before: None,
            limit: None,
            offset: None,
        })
}
