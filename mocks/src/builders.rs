//! Builder pattern implementations for easy test data construction
//! 
//! Provides fluent builders for:
//! - Task construction with sensible defaults
//! - NewTask and UpdateTask variants
//! - Filter construction for query testing

use task_core::{Task, TaskState, NewTask, UpdateTask, TaskFilter};
use chrono::{DateTime, Utc};

/// Builder for constructing Task instances in tests
pub struct TaskBuilder {
    task: Task,
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskBuilder {
    /// Create new builder with default values
    pub fn new() -> Self {
        Self {
            task: Task::new(
                1,
                "TEST-001".to_string(),
                "Test Task".to_string(),
                "A test task".to_string(),
                Some("test-agent".to_string()),
                TaskState::Created,
                Utc::now(),
                None,
            ),
        }
    }

    /// Set task ID
    pub fn with_id(mut self, id: i32) -> Self {
        self.task.id = id;
        self
    }

    /// Set task code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.task.code = code.into();
        self
    }

    /// Set task name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.task.name = name.into();
        self
    }

    /// Set task description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.task.description = description.into();
        self
    }

    /// Set task state
    pub fn with_state(mut self, state: TaskState) -> Self {
        self.task.state = state;
        // Set done_at if moving to Done state
        if state == TaskState::Done && self.task.done_at.is_none() {
            self.task.done_at = Some(Utc::now());
        }
        self
    }

    /// Set task owner
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.task.owner_agent_name = Some(owner.into());
        self
    }

    /// Set insertion timestamp
    pub fn with_inserted_at(mut self, inserted_at: DateTime<Utc>) -> Self {
        self.task.inserted_at = inserted_at;
        self
    }

    /// Set completion timestamp
    pub fn with_done_at(mut self, done_at: Option<DateTime<Utc>>) -> Self {
        self.task.done_at = done_at;
        self
    }

    /// Build the final Task
    pub fn build(self) -> Task {
        self.task
    }
}

/// Builder for constructing NewTask instances in tests
pub struct NewTaskBuilder {
    new_task: NewTask,
}

impl Default for NewTaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NewTaskBuilder {
    /// Create new builder with default values
    pub fn new() -> Self {
        Self {
            new_task: NewTask::new(
                "NEW-001".to_string(),
                "New Test Task".to_string(),
                "A new test task".to_string(),
                Some("test-agent".to_string()),
            ),
        }
    }

    /// Set code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.new_task.code = code.into();
        self
    }

    /// Set name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.new_task.name = name.into();
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.new_task.description = description.into();
        self
    }

    /// Set owner
    pub fn with_owner_agent_name(mut self, owner: impl Into<String>) -> Self {
        self.new_task.owner_agent_name = Some(owner.into());
        self
    }

    /// Build the final NewTask
    pub fn build(self) -> NewTask {
        self.new_task
    }
}

/// Builder for constructing UpdateTask instances in tests
pub struct UpdateTaskBuilder {
    update_task: UpdateTask,
}

impl Default for UpdateTaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateTaskBuilder {
    /// Create new builder with default values
    pub fn new() -> Self {
        Self {
            update_task: UpdateTask::default(),
        }
    }

    /// Set name update
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.update_task.name = Some(name.into());
        self
    }

    /// Set description update
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.update_task.description = Some(description.into());
        self
    }

    /// Set owner update
    pub fn with_owner_agent_name(mut self, owner: impl Into<String>) -> Self {
        self.update_task.owner_agent_name = Some(owner.into());
        self
    }

    /// Build the final UpdateTask
    pub fn build(self) -> UpdateTask {
        self.update_task
    }
}

/// Builder for constructing TaskFilter instances in tests
pub struct TaskFilterBuilder {
    filter: TaskFilter,
}

impl Default for TaskFilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskFilterBuilder {
    /// Create new builder with default values
    pub fn new() -> Self {
        Self {
            filter: TaskFilter::default(),
        }
    }

    /// Filter by owner
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.filter.owner = Some(owner.into());
        self
    }

    /// Filter by state
    pub fn with_state(mut self, state: TaskState) -> Self {
        self.filter.state = Some(state);
        self
    }

    /// Filter by date range from
    pub fn with_date_from(mut self, date_from: DateTime<Utc>) -> Self {
        self.filter.date_from = Some(date_from);
        self
    }

    /// Filter by date range to
    pub fn with_date_to(mut self, date_to: DateTime<Utc>) -> Self {
        self.filter.date_to = Some(date_to);
        self
    }

    /// Build the final TaskFilter
    pub fn build(self) -> TaskFilter {
        self.filter
    }
}