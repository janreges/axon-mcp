# Task Core Library

The foundational crate for the MCP Task Management Server, providing domain models, business logic, and trait interfaces that all other crates depend on.

## Overview

The `core` crate is designed to be pure business logic with zero I/O operations. It serves as the foundation for the entire MCP Task Management System by defining essential building blocks and interfaces.

## Key Components

### Domain Models
- **Task**: Core task representation with lifecycle state management
- **TaskState**: Enum defining task progression through the system
- **NewTask**: DTO for task creation
- **UpdateTask**: DTO for task modifications
- **TaskFilter**: Query parameters for task filtering

### Error Handling
- **TaskError**: Comprehensive error types covering all failure modes
- **Result**: Type alias for operations that can fail
- HTTP status code mapping for API responses

### Repository Interface
- **TaskRepository**: Async trait for data persistence operations
- **RepositoryStats**: Statistics and monitoring data
- Thread-safe design supporting concurrent access

### Protocol Interface
- **ProtocolHandler**: Async trait for MCP function implementations
- Parameter types for all 8 MCP functions
- Built-in validation logic

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
task-core = { path = "../core" }
```

### Basic Usage

```rust
use task_core::{Task, TaskState, NewTask, TaskError, Result};
use chrono::Utc;

// Create a new task
let new_task = NewTask {
    code: "FEAT-001".to_string(),
    name: "Implement user authentication".to_string(),
    description: "Add JWT-based authentication with role-based access control".to_string(),
    owner_agent_name: "backend-developer".to_string(),
};

// Work with task states
let task = Task {
    id: 42,
    code: "FEAT-001".to_string(),
    name: "Implement user authentication".to_string(),
    description: "Add JWT-based authentication with role-based access control".to_string(),
    owner_agent_name: "backend-developer".to_string(),
    state: TaskState::Created,
    inserted_at: Utc::now(),
    done_at: None,
};

// Validate state transitions
assert!(task.can_transition_to(TaskState::InProgress));
assert!(!task.can_transition_to(TaskState::Done)); // Invalid from Created
```

### Repository Implementation

```rust
use task_core::{TaskRepository, TaskFilter, Result};
use async_trait::async_trait;

struct MyRepository {
    // Your database connection, etc.
}

#[async_trait]
impl TaskRepository for MyRepository {
    async fn create(&self, task: NewTask) -> Result<Task> {
        // Your implementation
        todo!()
    }
    
    async fn get_by_id(&self, id: i32) -> Result<Option<Task>> {
        // Your implementation
        todo!()
    }
    
    // ... implement other methods
}
```

### Protocol Handler Implementation

```rust
use task_core::{ProtocolHandler, CreateTaskParams, Result};
use async_trait::async_trait;

struct MyHandler {
    repository: Box<dyn TaskRepository>,
}

#[async_trait]
impl ProtocolHandler for MyHandler {
    async fn create_task(&self, params: CreateTaskParams) -> Result<Task> {
        // Validate parameters
        params.validate()?;
        
        // Create via repository
        let new_task = NewTask {
            code: params.code,
            name: params.name,
            description: params.description,
            owner_agent_name: params.owner_agent_name,
        };
        
        self.repository.create(new_task).await
    }
    
    // ... implement other methods
}
```

## Task Lifecycle

Tasks progress through a defined state machine:

```
Created → InProgress → Review → Done → Archived
    ↓         ↓          ↓       ↓
  Blocked ←---+----------+-------+
```

### State Transitions

- **Created** → InProgress
- **InProgress** → Blocked, Review, Done
- **Blocked** → InProgress
- **Review** → InProgress, Done
- **Done** → Archived (via archive_task only)
- **Archived** → (no transitions allowed)

Use `Task::can_transition_to()` to validate transitions before attempting them.

## Error Handling

The crate provides comprehensive error types with helpful categorization:

```rust
use task_core::TaskError;

match error {
    TaskError::NotFound(msg) => {
        // Handle not found - maps to HTTP 404
        println!("Task not found: {}", msg);
    }
    TaskError::InvalidStateTransition(from, to) => {
        // Handle invalid transition - maps to HTTP 422
        println!("Cannot transition from {} to {}", from, to);
    }
    TaskError::Validation(msg) => {
        // Handle validation error - maps to HTTP 400
        println!("Validation failed: {}", msg);
    }
    // ... handle other error types
}

// Check error categories
if error.is_not_found() {
    // Handle not found case
}

// Get HTTP status code equivalent
let status = error.status_code(); // 404, 400, 422, etc.
```

## Architecture Principles

### Pure Business Logic
- No I/O operations or external dependencies
- All logic is testable and deterministic
- Clear separation of concerns

### Async by Default
- All traits use `async fn` for future compatibility
- Thread-safe with `Send + Sync` bounds
- Designed for high-concurrency scenarios

### Trait-Based Design
- Repository trait abstracts data persistence
- Protocol trait abstracts MCP handling
- Enables dependency injection and testing

### Comprehensive Validation
- Built-in validation for all operations
- State transition enforcement
- Parameter validation with helpful error messages

## Testing

Run the test suite:

```bash
cd core
cargo test
```

The crate includes comprehensive unit tests covering:
- State transition validation
- Error creation and categorization
- Parameter validation
- Trait bounds and object safety

## Documentation

Generate and view documentation:

```bash
cargo doc --open
```

All public APIs are documented with examples and detailed explanations.

## Dependencies

The crate has minimal dependencies:
- `chrono`: Date/time handling with serde support
- `serde`: Serialization for all data types
- `thiserror`: Error handling with Display derivation
- `async-trait`: Async trait support

No runtime dependencies or I/O libraries - this is pure business logic.

## Version

Current version: `0.1.0`

Access the version at runtime:
```rust
use task_core::VERSION;
println!("Core version: {}", VERSION);
```