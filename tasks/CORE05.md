# CORE05: Extend TaskRepository Trait - Messages

## Objective
Extend the TaskRepository trait in the core crate to include methods for managing task messages, enabling agents to communicate about tasks.

## Current State
The TaskRepository trait in `core/src/repository.rs` currently has basic CRUD operations and some MCP v2 methods.

## Required Changes

### 1. Add Message-Related Types to Core
First, ensure these types exist in `core/src/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMessage {
    pub id: i32,
    pub task_code: String,
    pub author_agent_name: String,
    pub message_type: MessageType,
    pub created_at: DateTime<Utc>,
    pub content: String,
    pub reply_to_message_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTaskMessage {
    pub task_code: String,
    pub author_agent_name: String,
    pub message_type: MessageType,
    pub content: String,
    pub reply_to_message_id: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageFilter {
    pub task_code: Option<String>,
    pub message_types: Vec<MessageType>,
    pub since: Option<DateTime<Utc>>,
    pub author_agent_name: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSearchQuery {
    pub query: String,
    pub task_codes: Option<Vec<String>>,
    pub message_types: Vec<MessageType>,
    pub limit: Option<i32>,
}
```

### 2. Extend TaskRepository Trait
Add to `core/src/repository.rs`:

```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Task Messages Methods =====
    
    /// Add a message to a task
    async fn add_task_message(&self, message: NewTaskMessage) -> Result<TaskMessage>;
    
    /// Get messages for a specific task with filtering
    async fn get_task_messages(&self, filter: MessageFilter) -> Result<Vec<TaskMessage>>;
    
    /// Get a specific message by ID
    async fn get_message_by_id(&self, message_id: i32) -> Result<Option<TaskMessage>>;
    
    /// Search messages across tasks
    async fn search_task_messages(&self, query: MessageSearchQuery) -> Result<Vec<TaskMessage>>;
    
    /// Get message thread (message and all replies)
    async fn get_message_thread(&self, message_id: i32) -> Result<Vec<TaskMessage>>;
    
    /// Count messages for a task by type
    async fn count_task_messages(&self, task_code: &str) -> Result<MessageCountByType>;
    
    /// Mark messages as read by an agent (for future notification system)
    async fn mark_messages_read(&self, task_code: &str, agent_name: &str) -> Result<()>;
    
    /// Get unread message count for an agent
    async fn get_unread_count(&self, agent_name: &str) -> Result<i32>;
}
```

### 3. Add Supporting Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCountByType {
    pub total: i32,
    pub by_type: HashMap<MessageType, i32>,
    pub unresolved_questions: i32,
    pub active_blockers: i32,
}

impl Default for MessageCountByType {
    fn default() -> Self {
        Self {
            total: 0,
            by_type: HashMap::new(),
            unresolved_questions: 0,
            active_blockers: 0,
        }
    }
}
```

### 4. Add Validation Methods
Add to `NewTaskMessage` implementation:

```rust
impl NewTaskMessage {
    pub fn validate(&self) -> Result<()> {
        // Validate task code format
        if self.task_code.is_empty() {
            return Err(TaskError::Validation("Task code cannot be empty".to_string()));
        }
        
        // Validate agent name format (kebab-case)
        if !self.author_agent_name.chars().all(|c| c.is_lowercase() || c == '-' || c.is_numeric()) {
            return Err(TaskError::Validation(
                "Agent name must be in kebab-case format".to_string()
            ));
        }
        
        // Validate content length
        if self.content.is_empty() {
            return Err(TaskError::Validation("Message content cannot be empty".to_string()));
        }
        
        if self.content.len() > 10000 {
            return Err(TaskError::Validation(
                "Message content cannot exceed 10000 characters".to_string()
            ));
        }
        
        Ok(())
    }
}
```

### 5. Add Helper Methods to MessageFilter
```rust
impl MessageFilter {
    /// Create a filter for a specific task
    pub fn for_task(task_code: &str) -> Self {
        Self {
            task_code: Some(task_code.to_string()),
            ..Default::default()
        }
    }
    
    /// Create a filter for specific message types
    pub fn by_types(types: Vec<MessageType>) -> Self {
        Self {
            message_types: types,
            ..Default::default()
        }
    }
    
    /// Filter for messages that need responses
    pub fn needs_response() -> Self {
        Self {
            message_types: vec![
                MessageType::Question,
                MessageType::Blocker,
                MessageType::Review,
            ],
            ..Default::default()
        }
    }
    
    /// Add pagination
    pub fn with_pagination(mut self, limit: i32, offset: i32) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
}
```

### 6. Protocol Handler Trait Extension
Add corresponding methods to `ProtocolHandler` trait:

```rust
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    // ... existing methods ...
    
    // Task Messages
    async fn add_task_message(&self, params: AddTaskMessageParams) -> Result<TaskMessage>;
    async fn get_task_messages(&self, params: GetTaskMessagesParams) -> Result<Vec<TaskMessage>>;
    async fn search_task_messages(&self, params: SearchTaskMessagesParams) -> Result<Vec<TaskMessage>>;
}

// Parameter types
#[derive(Debug, Deserialize)]
pub struct AddTaskMessageParams {
    pub task_code: String,
    pub author_agent_name: String,
    pub message_type: MessageType,
    pub content: String,
    pub reply_to_message_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct GetTaskMessagesParams {
    pub task_code: String,
    pub requesting_agent_name: String,
    pub message_types: Vec<MessageType>,
    pub since: Option<String>, // ISO 8601 datetime
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SearchTaskMessagesParams {
    pub query: String,
    pub requesting_agent_name: String,
    pub task_codes: Option<Vec<String>>,
    pub message_types: Vec<MessageType>,
    pub limit: Option<i32>,
}
```

## Files to Modify
- `core/src/repository.rs` - Add message methods to trait
- `core/src/protocol.rs` - Add protocol handler methods
- `core/src/models.rs` - Add message-related types
- `core/src/error.rs` - Add any new error variants if needed

## Testing Requirements
1. Mock implementations for all new methods
2. Unit tests for validation methods
3. Tests for filter builders
4. Integration tests will be in database crate

## Notes
- Message IDs are auto-generated by the database
- Agent names must be validated against the agent registry
- Consider rate limiting for message creation in the future
- Messages are immutable once created (no edit functionality)