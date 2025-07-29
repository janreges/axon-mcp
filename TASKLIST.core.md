# Task List: `core` Crate

**Owner Agents**: rust-architect + backend-developer  
**Purpose**: Define domain models, business logic, and trait interfaces that all other crates depend on.

## Critical Requirements

This crate MUST:
- Have ZERO I/O operations (pure business logic)
- Compile with `--no-default-features`
- Define all trait interfaces that other crates will implement
- Provide comprehensive error types for the entire system

## Phase 1: Project Setup ✓ Required

- [ ] Create `core/` directory
- [ ] Create `core/Cargo.toml` with dependencies:
  ```toml
  [package]
  name = "core"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  serde = { version = "1.0", features = ["derive"] }
  chrono = { version = "0.4", features = ["serde"] }
  thiserror = "1.0"
  async-trait = "0.1"
  ```
- [ ] Create directory structure:
  ```
  core/
  ├── src/
  │   ├── lib.rs
  │   ├── models.rs
  │   ├── error.rs
  │   ├── repository.rs
  │   ├── protocol.rs
  │   └── validation.rs
  ```

## Phase 2: Domain Models Implementation ✓ Required

### Task 1: Create Core Models (`src/models.rs`)
- [ ] Implement `Task` struct with all fields from ARCHITECTURE.md
- [ ] Implement `TaskState` enum with all variants
- [ ] Implement `NewTask` DTO for task creation
- [ ] Implement `UpdateTask` DTO for task updates
- [ ] Implement `TaskFilter` for query operations
- [ ] Add comprehensive `#[derive(...)]` attributes
- [ ] Add documentation comments for all public types

### Task 2: Create Error Types (`src/error.rs`)
- [ ] Implement `TaskError` enum with all variants:
  - `NotFound(String)`
  - `InvalidStateTransition(TaskState, TaskState)`
  - `DuplicateCode(String)`
  - `Validation(String)`
  - `Database(String)`
  - `Protocol(String)`
- [ ] Define `Result<T>` type alias
- [ ] Implement Display trait with meaningful messages
- [ ] Add conversion methods for common error scenarios

### Task 3: Create Repository Trait (`src/repository.rs`)
- [ ] Define `TaskRepository` trait with `#[async_trait]`
- [ ] Implement all methods EXACTLY as specified:
  ```rust
  async fn create(&self, task: NewTask) -> Result<Task>;
  async fn update(&self, id: i32, updates: UpdateTask) -> Result<Task>;
  async fn set_state(&self, id: i32, state: TaskState) -> Result<Task>;
  async fn get_by_id(&self, id: i32) -> Result<Option<Task>>;
  async fn get_by_code(&self, code: &str) -> Result<Option<Task>>;
  async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>>;
  async fn assign(&self, id: i32, new_owner: &str) -> Result<Task>;
  async fn archive(&self, id: i32) -> Result<Task>;
  ```
- [ ] Add trait bounds: `Send + Sync`
- [ ] Add comprehensive documentation for each method

### Task 4: Create Protocol Trait (`src/protocol.rs`)
- [ ] Define `ProtocolHandler` trait with `#[async_trait]`
- [ ] Implement all MCP parameter types:
  - `CreateTaskParams`
  - `UpdateTaskParams`
  - `SetStateParams`
  - `ListTasksParams`
  - `AssignTaskParams`
- [ ] Implement all protocol methods EXACTLY as specified
- [ ] Add trait bounds: `Send + Sync`
- [ ] Add comprehensive documentation

### Task 5: Create Validation Logic (`src/validation.rs`)
- [ ] Implement `Task::can_transition_to()` method
- [ ] Define valid state transitions:
  - Created → InProgress
  - InProgress → Blocked, Review, Done
  - Blocked → InProgress
  - Review → InProgress, Done
  - Done → Archived
- [ ] Add validation helper methods:
  - `validate_task_code(code: &str) -> Result<()>`
  - `validate_agent_name(name: &str) -> Result<()>`
- [ ] Add comprehensive tests for state transitions

### Task 6: Create Main Library File (`src/lib.rs`)
- [ ] Export all public modules
- [ ] Re-export commonly used types at crate root
- [ ] Add crate-level documentation
- [ ] Configure feature flags if needed

## Phase 3: Testing ✓ Required

- [ ] Create unit tests for state transitions
- [ ] Create unit tests for validation logic
- [ ] Create unit tests for error conversions
- [ ] Ensure 100% test coverage for business logic
- [ ] Add property-based tests for models
- [ ] Create test fixtures in `tests/` directory

## Phase 4: Documentation ✓ Required

- [ ] Add rustdoc comments for ALL public items
- [ ] Create examples in documentation
- [ ] Add module-level documentation
- [ ] Generate and review rustdoc output
- [ ] Create `README.md` for the crate

## Public Interface Checklist ✓ MUST MATCH ARCHITECTURE.md

### Models (`models.rs`)
- [ ] `Task` struct with exact fields
- [ ] `TaskState` enum with exact variants
- [ ] `NewTask` struct
- [ ] `UpdateTask` struct
- [ ] `TaskFilter` struct

### Errors (`error.rs`)
- [ ] `TaskError` enum with all variants
- [ ] `Result<T>` type alias

### Repository Trait (`repository.rs`)
- [ ] `TaskRepository` trait with 8 methods
- [ ] All method signatures match exactly
- [ ] Proper async and error handling

### Protocol Trait (`protocol.rs`)
- [ ] `ProtocolHandler` trait with 8 methods
- [ ] All parameter types defined
- [ ] All method signatures match exactly

### Validation (`validation.rs`)
- [ ] `can_transition_to()` method on Task
- [ ] State transition rules implemented

## Quality Checklist

- [ ] No compiler warnings
- [ ] All clippy lints pass
- [ ] Code formatted with rustfmt
- [ ] No unsafe code
- [ ] No panics in library code
- [ ] All errors handled properly
- [ ] No hardcoded values

## Communication Points

Use `./log.sh` to communicate:
- When trait interfaces are ready for review
- When breaking changes are needed
- When implementation is complete
- When help is needed from other teams

Example:
```bash
./log.sh "RUST-ARCHITECT → ALL: Core trait interfaces ready for review"
./log.sh "BACKEND-DEVELOPER → DATABASE: TaskRepository trait finalized"
```

## Success Criteria

1. Crate compiles independently
2. All tests pass
3. Zero external dependencies beyond approved list
4. 100% API compatibility with ARCHITECTURE.md
5. Comprehensive documentation
6. Other crates can depend on it successfully