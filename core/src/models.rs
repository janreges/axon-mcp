use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Core task representation in the MCP Task Management System.
/// 
/// A task represents a unit of work that can be tracked through its lifecycle,
/// assigned to agents, and managed via the MCP protocol. Each task has a unique
/// numeric ID and human-readable code for easy reference.
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::models::{Task, TaskState};
/// use chrono::Utc;
/// 
/// let task = Task {
///     id: 42,
///     code: "FEAT-001".to_string(),
///     name: "Implement user authentication".to_string(),
///     description: "Add JWT-based auth with role-based access control".to_string(),
///     owner_agent_name: "backend-developer".to_string(),
///     state: TaskState::Created,
///     inserted_at: Utc::now(),
///     done_at: None,
/// };
/// 
/// // Check if task can transition to InProgress
/// assert!(task.can_transition_to(TaskState::InProgress));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    /// Auto-increment primary key
    pub id: i32,
    /// Human-readable identifier (e.g., "ARCH-01", "DB-15")
    pub code: String,
    /// Brief task title
    pub name: String,
    /// Detailed task requirements
    pub description: String,
    /// Assigned agent identifier
    pub owner_agent_name: String,
    /// Current lifecycle state
    pub state: TaskState,
    /// Creation timestamp
    pub inserted_at: DateTime<Utc>,
    /// Completion timestamp
    pub done_at: Option<DateTime<Utc>>,
}

/// Task lifecycle states defining the progression of work.
/// 
/// Tasks move through a defined state machine with validated transitions.
/// The typical flow is: Created → InProgress → Review → Done → Archived,
/// with Blocked as a temporary state that can occur during InProgress.
/// 
/// # State Transitions
/// 
/// - `Created` → `InProgress`
/// - `InProgress` → `Blocked`, `Review`, `Done`  
/// - `Blocked` → `InProgress`
/// - `Review` → `InProgress`, `Done`
/// - `Done` → `Archived` (via archive_task only)
/// - `Archived` → (no transitions allowed)
/// 
/// # Examples
/// 
/// ```rust
/// use task_core::models::{Task, TaskState};
/// use chrono::Utc;
/// 
/// let task = Task {
///     id: 1,
///     code: "TEST-01".to_string(),
///     name: "Test Task".to_string(),
///     description: "A test task".to_string(),
///     owner_agent_name: "test-agent".to_string(),
///     state: TaskState::Created,
///     inserted_at: Utc::now(),
///     done_at: None,
/// };
/// 
/// // Check valid transitions
/// if task.can_transition_to(TaskState::InProgress) {
///     // Safe to move to InProgress
/// }
/// ```
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskState {
    /// Newly created task
    Created,
    /// Task is actively being worked on
    InProgress,
    /// Task is blocked and cannot proceed
    Blocked,
    /// Task is ready for review
    Review,
    /// Task has been completed
    Done,
    /// Task has been archived
    Archived,
}

/// Data transfer object for creating new tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    /// Human-readable identifier (e.g., "ARCH-01", "DB-15")
    pub code: String,
    /// Brief task title
    pub name: String,
    /// Detailed task requirements
    pub description: String,
    /// Assigned agent identifier
    pub owner_agent_name: String,
}

/// Data transfer object for updating existing tasks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTask {
    /// Optional new task name
    pub name: Option<String>,
    /// Optional new task description
    pub description: Option<String>,
    /// Optional new owner agent
    pub owner_agent_name: Option<String>,
}

/// Filter criteria for querying tasks.
///
/// All fields are optional to support flexible querying.
/// When multiple fields are specified, they are combined with AND logic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    /// Filter by owner agent name
    pub owner: Option<String>,
    
    /// Filter by task state
    pub state: Option<TaskState>,
    
    /// Filter tasks created on or after this date
    pub date_from: Option<DateTime<Utc>>,
    
    /// Filter tasks created on or before this date
    pub date_to: Option<DateTime<Utc>>,
}

impl Task {
    /// Check if the task can transition to the given state
    pub fn can_transition_to(&self, new_state: TaskState) -> bool {
        use TaskState::*;
        
        match (self.state, new_state) {
            // Can't transition to the same state
            (current, new) if current == new => false,
            
            // Valid transitions from Created
            (Created, InProgress) => true,
            
            // Valid transitions from InProgress
            (InProgress, Blocked | Review | Done) => true,
            
            // Valid transitions from Blocked
            (Blocked, InProgress) => true,
            
            // Valid transitions from Review
            (Review, InProgress | Done) => true,
            
            // Valid transitions from Done
            (Done, Archived) => true,
            
            // No valid transitions from Archived
            (Archived, _) => false,
            
            // All other transitions are invalid
            _ => false,
        }
    }
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::Created => write!(f, "Created"),
            TaskState::InProgress => write!(f, "InProgress"), 
            TaskState::Blocked => write!(f, "Blocked"),
            TaskState::Review => write!(f, "Review"),
            TaskState::Done => write!(f, "Done"),
            TaskState::Archived => write!(f, "Archived"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_state_transitions() {
        let task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        };

        // Created -> InProgress
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Archived));

        // InProgress -> Blocked, Review, Done
        let mut task = task;
        task.state = TaskState::InProgress;
        assert!(task.can_transition_to(TaskState::Blocked));
        assert!(task.can_transition_to(TaskState::Review));
        assert!(task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Blocked -> InProgress
        task.state = TaskState::Blocked;
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Review -> InProgress, Done
        task.state = TaskState::Review;
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(task.can_transition_to(TaskState::Done));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Archived));

        // Done -> Archived
        task.state = TaskState::Done;
        assert!(task.can_transition_to(TaskState::Archived));
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));

        // Archived -> nothing
        task.state = TaskState::Archived;
        assert!(!task.can_transition_to(TaskState::Created));
        assert!(!task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Blocked));
        assert!(!task.can_transition_to(TaskState::Review));
        assert!(!task.can_transition_to(TaskState::Done));
    }

    #[test]
    fn test_no_same_state_transition() {
        let task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::InProgress,
            inserted_at: Utc::now(),
            done_at: None,
        };

        // Cannot transition to the same state
        assert!(!task.can_transition_to(TaskState::InProgress));
    }
}