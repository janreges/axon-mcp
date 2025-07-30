# CORE01: Define Enhanced TaskState Enum

## Objective
Extend the existing TaskState enum in the core crate to include new MCP v2 states that support advanced multi-agent workflows.

## Current State
```rust
// core/src/models.rs
pub enum TaskState {
    Created,
    InProgress,
    Blocked,
    Review,
    Done,
    Archived,
}
```

## Required Changes

### 1. Add New Task States
Add the following new states to the TaskState enum:

```rust
pub enum TaskState {
    // Existing states
    Created,
    InProgress,
    Blocked,
    Review,
    Done,
    Archived,
    
    // New MCP v2 states
    PendingDecomposition,    // Task needs to be broken down into subtasks
    PendingHandoff,          // Waiting for agent handoff  
    Quarantined,             // Too many failures, needs human review
    WaitingForDependency,    // Blocked on other tasks completing
}
```

### 2. Update Display Implementation
Update the Display trait implementation to handle new states:

```rust
impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ... existing matches ...
            TaskState::PendingDecomposition => write!(f, "PendingDecomposition"),
            TaskState::PendingHandoff => write!(f, "PendingHandoff"),
            TaskState::Quarantined => write!(f, "Quarantined"),
            TaskState::WaitingForDependency => write!(f, "WaitingForDependency"),
        }
    }
}
```

### 3. Update State Transition Logic
Modify the `can_transition_to` method to handle new state transitions:

```rust
impl Task {
    pub fn can_transition_to(&self, new_state: TaskState) -> bool {
        use TaskState::*;
        match (self.state, new_state) {
            // Existing transitions...
            
            // New transitions
            (Created, PendingDecomposition) => true,
            (PendingDecomposition, Created) => true, // After decomposition
            (InProgress, PendingHandoff) => true,
            (PendingHandoff, InProgress) => true, // When handoff accepted
            (_, Quarantined) => true, // Any state can be quarantined
            (Quarantined, Created) => true, // Reset after human review
            (Created, WaitingForDependency) => true,
            (WaitingForDependency, Created) => true, // When dependencies met
            _ => false,
        }
    }
}
```

### 4. Update Serialization
Ensure the new states are properly handled in serde serialization/deserialization.

## Files to Modify
- `core/src/models.rs` - Add new enum variants and update implementations
- `core/src/error.rs` - Update InvalidStateTransition error messages if needed

## Testing Requirements
1. Add unit tests for new state transitions
2. Verify serialization/deserialization works for new states
3. Test database persistence of new states
4. Ensure all existing tests still pass

## Dependencies
- No new crate dependencies required
- Must maintain backward compatibility with existing states

## Migration Considerations
- Existing tasks in the database will retain their current states
- No data migration needed as we're only adding new states