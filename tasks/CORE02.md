# CORE02: Define MessageType and KnowledgeType Enums

## Objective
Create new enums for task messages and knowledge objects that enable rich communication between agents in the MCP v2 system.

## Implementation Details

### 1. Create MessageType Enum
Create in `core/src/models.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Comment,         // General comment
    Question,        // Question that needs answering
    Update,          // Status or progress update
    Blocker,         // Issue preventing progress
    Solution,        // Solution or workaround
    Review,          // Code/work review comment
    Handoff,         // Handoff related message
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageType::Comment => write!(f, "comment"),
            MessageType::Question => write!(f, "question"),
            MessageType::Update => write!(f, "update"),
            MessageType::Blocker => write!(f, "blocker"),
            MessageType::Solution => write!(f, "solution"),
            MessageType::Review => write!(f, "review"),
            MessageType::Handoff => write!(f, "handoff"),
        }
    }
}
```

### 2. Create KnowledgeType Enum
Add to `core/src/models.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeType {
    Note,              // General observation or comment
    Decision,          // Important decision with rationale
    Question,          // Question that needs answering
    Answer,            // Response to a question
    Handoff,           // Formal handoff package
    StepOutput,        // Output from a workflow step
    Blocker,           // Issue preventing progress
    Resolution,        // Solution to a blocker
    Artifact,          // Reference to external resource
}

impl fmt::Display for KnowledgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KnowledgeType::Note => write!(f, "note"),
            KnowledgeType::Decision => write!(f, "decision"),
            KnowledgeType::Question => write!(f, "question"),
            KnowledgeType::Answer => write!(f, "answer"),
            KnowledgeType::Handoff => write!(f, "handoff"),
            KnowledgeType::StepOutput => write!(f, "step_output"),
            KnowledgeType::Blocker => write!(f, "blocker"),
            KnowledgeType::Resolution => write!(f, "resolution"),
            KnowledgeType::Artifact => write!(f, "artifact"),
        }
    }
}
```

### 3. Create Visibility Enum
Add to `core/src/models.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,    // Visible to all agents
    Team,      // Visible to agents with shared capabilities
    Private,   // Only visible to author and task owner
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public => write!(f, "public"),
            Visibility::Team => write!(f, "team"),
            Visibility::Private => write!(f, "private"),
        }
    }
}
```

### 4. Create Helper Methods
Add utility methods for filtering and validation:

```rust
impl MessageType {
    /// Returns true if this message type typically requires a response
    pub fn requires_response(&self) -> bool {
        matches!(self, MessageType::Question | MessageType::Blocker | MessageType::Review)
    }
    
    /// Returns true if this is a blocking message type
    pub fn is_blocking(&self) -> bool {
        matches!(self, MessageType::Blocker)
    }
}

impl KnowledgeType {
    /// Returns true if this knowledge type represents a question
    pub fn is_question(&self) -> bool {
        matches!(self, KnowledgeType::Question | KnowledgeType::Blocker)
    }
    
    /// Returns true if this knowledge type represents an answer/resolution
    pub fn is_resolution(&self) -> bool {
        matches!(self, KnowledgeType::Answer | KnowledgeType::Resolution)
    }
}
```

### 5. Add Database Conversion Traits
Implement conversion to/from database strings:

```rust
impl TryFrom<&str> for MessageType {
    type Error = TaskError;
    
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "comment" => Ok(MessageType::Comment),
            "question" => Ok(MessageType::Question),
            "update" => Ok(MessageType::Update),
            "blocker" => Ok(MessageType::Blocker),
            "solution" => Ok(MessageType::Solution),
            "review" => Ok(MessageType::Review),
            "handoff" => Ok(MessageType::Handoff),
            _ => Err(TaskError::Validation(format!("Invalid message type: {}", value))),
        }
    }
}

// Similar implementations for KnowledgeType and Visibility
```

## Files to Create/Modify
- `core/src/models.rs` - Add all new enum definitions
- Consider creating `core/src/models/messages.rs` if the file gets too large

## Testing Requirements
1. Unit tests for all Display implementations
2. Unit tests for TryFrom conversions
3. Tests for helper methods (requires_response, is_blocking, etc.)
4. Serialization/deserialization tests
5. Database round-trip tests

## Re-exports
Add to `core/src/lib.rs`:
```rust
pub use models::{MessageType, KnowledgeType, Visibility};
```

## Documentation
Add comprehensive rustdoc comments explaining:
- When to use each message type
- The purpose of each knowledge type
- Visibility rules and implications