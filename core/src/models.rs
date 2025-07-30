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
///     workflow_definition_id: None,
///     workflow_cursor: None,
///     priority_score: 5.0,
///     parent_task_id: None,
///     failure_count: 0,
///     required_capabilities: vec!["auth".to_string(), "jwt".to_string()],
///     estimated_effort: Some(120), // 2 hours
///     confidence_threshold: 0.8,
/// };
/// 
/// // Check if task can transition to InProgress
/// assert!(task.can_transition_to(TaskState::InProgress));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    
    // MCP v2 Extensions
    /// Workflow definition ID for structured task execution
    pub workflow_definition_id: Option<i32>,
    /// Current position in workflow execution
    pub workflow_cursor: Option<String>,
    /// Task priority score (0.0 = lowest, 10.0 = highest)
    pub priority_score: f64,
    /// Parent task for hierarchical task structures
    pub parent_task_id: Option<i32>,
    /// Number of times this task has failed
    pub failure_count: i32,
    /// Required agent capabilities for task execution
    pub required_capabilities: Vec<String>,
    /// Estimated effort in minutes
    pub estimated_effort: Option<i32>,
    /// Confidence threshold for task completion (0.0-1.0)
    pub confidence_threshold: f64,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewTask {
    /// Human-readable identifier (e.g., "ARCH-01", "DB-15")
    pub code: String,
    /// Brief task title
    pub name: String,
    /// Detailed task requirements
    pub description: String,
    /// Assigned agent identifier
    pub owner_agent_name: String,
    
    // MCP v2 Extensions
    /// Workflow definition ID for structured task execution
    pub workflow_definition_id: Option<i32>,
    /// Task priority score (0.0 = lowest, 10.0 = highest)
    #[serde(default = "default_priority_score")]
    pub priority_score: f64,
    /// Parent task for hierarchical task structures
    pub parent_task_id: Option<i32>,
    /// Required agent capabilities for task execution
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    /// Estimated effort in minutes
    pub estimated_effort: Option<i32>,
    /// Confidence threshold for task completion (0.0-1.0)
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f64,
}

fn default_priority_score() -> f64 {
    5.0 // Medium priority
}

fn default_confidence_threshold() -> f64 {
    0.8 // 80% confidence threshold
}

impl NewTask {
    /// Create a new NewTask with default MCP v2 values (for backward compatibility)
    pub fn new(
        code: String,
        name: String,
        description: String,
        owner_agent_name: String,
    ) -> Self {
        Self {
            code,
            name,
            description,
            owner_agent_name,
            workflow_definition_id: None,
            priority_score: 5.0,
            parent_task_id: None,
            required_capabilities: vec![],
            estimated_effort: None,
            confidence_threshold: 0.8,
        }
    }
}

/// Data transfer object for updating existing tasks
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct UpdateTask {
    /// Optional new task name
    pub name: Option<String>,
    /// Optional new task description
    pub description: Option<String>,
    /// Optional new owner agent
    pub owner_agent_name: Option<String>,
    
    // MCP v2 Extensions
    /// Optional workflow definition ID
    pub workflow_definition_id: Option<Option<i32>>,
    /// Optional workflow cursor position
    pub workflow_cursor: Option<Option<String>>,
    /// Optional task priority score
    pub priority_score: Option<f64>,
    /// Optional parent task ID
    pub parent_task_id: Option<Option<i32>>,
    /// Optional required capabilities
    pub required_capabilities: Option<Vec<String>>,
    /// Optional estimated effort
    pub estimated_effort: Option<Option<i32>>,
    /// Optional confidence threshold
    pub confidence_threshold: Option<f64>,
}

impl UpdateTask {
    /// Create a new UpdateTask with basic fields (for backward compatibility)
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create UpdateTask with name, description, and owner (common pattern)
    pub fn with_basic_fields(
        name: Option<String>,
        description: Option<String>,
        owner_agent_name: Option<String>,
    ) -> Self {
        Self {
            name,
            description,
            owner_agent_name,
            ..Default::default()
        }
    }
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
    
    /// Filter tasks completed on or after this date
    pub completed_after: Option<DateTime<Utc>>,
    
    /// Filter tasks completed on or before this date
    pub completed_before: Option<DateTime<Utc>>,
    
    /// Maximum number of tasks to return (for pagination)
    pub limit: Option<u32>,
    
    /// Number of tasks to skip (for pagination)
    pub offset: Option<u32>,
}

// MCP v2 New Entity Types

/// Knowledge object for storing and sharing information between agents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeObject {
    /// Auto-increment primary key
    pub id: i32,
    /// Human-readable identifier
    pub key: String,
    /// Knowledge content (JSON or structured data)
    pub value: serde_json::Value,
    /// Content type (e.g., "text/plain", "application/json", "code/rust")
    pub content_type: String,
    /// Tags for categorization and search
    pub tags: Vec<String>,
    /// Creating agent identifier
    pub created_by_agent: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Knowledge version for change tracking
    pub version: i32,
}

/// Message between agents during task execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskMessage {
    /// Auto-increment primary key
    pub id: i32,
    /// Associated task ID
    pub task_id: i32,
    /// Sending agent identifier
    pub from_agent: String,
    /// Receiving agent identifier (None for broadcast)
    pub to_agent: Option<String>,
    /// Message content
    pub content: String,
    /// Message type (info, warning, error, request, response)
    pub message_type: MessageType,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether message has been read
    pub read: bool,
}

/// Message types for agent communication
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageType {
    /// Informational message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
    /// Request for action or information
    Request,
    /// Response to a request
    Response,
}

/// Agent profile and capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentProfile {
    /// Agent identifier (primary key)
    pub name: String,
    /// Human-readable display name
    pub display_name: String,
    /// Agent description
    pub description: String,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Agent specializations
    pub specializations: Vec<String>,
    /// Current agent status
    pub status: AgentStatus,
    /// Last heartbeat timestamp
    pub last_heartbeat: DateTime<Utc>,
    /// Agent registration timestamp
    pub registered_at: DateTime<Utc>,
    /// Current load score (0.0 = idle, 1.0 = fully loaded)
    pub load_score: f64,
}

/// Agent status enumeration
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is online and available
    Online,
    /// Agent is busy but can accept low-priority tasks
    Busy,
    /// Agent is offline or unavailable
    Offline,
    /// Agent has encountered an error
    Error,
}

/// Workflow definition for structured task execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowDefinition {
    /// Auto-increment primary key
    pub id: i32,
    /// Workflow name
    pub name: String,
    /// Workflow description
    pub description: String,
    /// Workflow steps (JSON array)
    pub steps: serde_json::Value,
    /// Required capabilities for workflow execution
    pub required_capabilities: Vec<String>,
    /// Workflow version
    pub version: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Whether workflow is active
    pub active: bool,
}

/// System event for audit and monitoring
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemEvent {
    /// Auto-increment primary key
    pub id: i32,
    /// Event type (task_created, task_completed, agent_heartbeat, etc.)
    pub event_type: String,
    /// Related entity ID (task_id, agent_name, etc.)
    pub entity_id: Option<String>,
    /// Event data (JSON)
    pub data: serde_json::Value,
    /// Agent that triggered the event
    pub triggered_by: Option<String>,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event severity level
    pub severity: EventSeverity,
}

/// Event severity levels
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventSeverity {
    /// Informational event
    Info,
    /// Warning event
    Warning,
    /// Error event
    Error,
    /// Critical system event
    Critical,
}

/// Work session tracking for time management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkSession {
    /// Auto-increment primary key
    pub id: i32,
    /// Associated task ID
    pub task_id: i32,
    /// Agent working on the task
    pub agent_name: String,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Session end time (None if still active)
    pub ended_at: Option<DateTime<Utc>>,
    /// Work session notes
    pub notes: Option<String>,
    /// Productivity score (0.0-1.0)
    pub productivity_score: Option<f64>,
}

impl Task {
    /// Create a new Task with default MCP v2 values (for backward compatibility)
    pub fn new(
        id: i32,
        code: String,
        name: String,
        description: String,
        owner_agent_name: String,
        state: TaskState,
        inserted_at: DateTime<Utc>,
        done_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            code,
            name,
            description,
            owner_agent_name,
            state,
            inserted_at,
            done_at,
            workflow_definition_id: None,
            workflow_cursor: None,
            priority_score: 5.0,
            parent_task_id: None,
            failure_count: 0,
            required_capabilities: vec![],
            estimated_effort: None,
            confidence_threshold: 0.8,
        }
    }

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
            workflow_definition_id: None,
            workflow_cursor: None,
            priority_score: 5.0,
            parent_task_id: None,
            failure_count: 0,
            required_capabilities: vec![],
            estimated_effort: None,
            confidence_threshold: 0.8,
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
            workflow_definition_id: None,
            workflow_cursor: None,
            priority_score: 5.0,
            parent_task_id: None,
            failure_count: 0,
            required_capabilities: vec![],
            estimated_effort: None,
            confidence_threshold: 0.8,
        };

        // Cannot transition to the same state
        assert!(!task.can_transition_to(TaskState::InProgress));
    }
}