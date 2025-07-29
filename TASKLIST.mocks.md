# Task List: `mocks` Crate

**Owner Agent**: qa-tester  
**Purpose**: Provide mock implementations and test utilities for all other crates. This is a dev-dependency only crate.

## Critical Requirements

This crate MUST:
- Provide a complete mock implementation of `TaskRepository`
- Create realistic test data generators
- Offer test assertion helpers
- Support property-based testing
- Be usable by all other crates for testing
- NEVER be included in production builds

## Phase 1: Project Setup ✓ Required

- [ ] Create `mocks/` directory
- [ ] Create `mocks/Cargo.toml` with dependencies:
  ```toml
  [package]
  name = "mocks"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  core = { path = "../core" }
  tokio = { version = "1.0", features = ["sync", "macros"] }
  async-trait = "0.1"
  parking_lot = "0.12"
  chrono = { version = "0.4", features = ["serde"] }
  fake = { version = "2.9", features = ["derive", "chrono"] }
  rand = "0.8"
  proptest = "1.0"

  [dev-dependencies]
  tokio-test = "0.4"
  ```
- [ ] Create directory structure:
  ```
  mocks/
  ├── src/
  │   ├── lib.rs
  │   ├── repository.rs
  │   ├── fixtures.rs
  │   ├── assertions.rs
  │   ├── builders.rs
  │   └── generators.rs
  └── tests/
  ```

## Phase 2: Mock Repository Implementation ✓ Required

### Task 1: Create Mock Repository (`src/repository.rs`)
- [ ] Define `MockTaskRepository` struct:
  ```rust
  pub struct MockTaskRepository {
      tasks: Arc<Mutex<HashMap<i32, Task>>>,
      next_id: Arc<AtomicI32>,
      error_injection: Arc<Mutex<Option<TaskError>>>,
      call_history: Arc<Mutex<Vec<String>>>,
  }
  ```
- [ ] Implement constructor variants:
  ```rust
  impl MockTaskRepository {
      /// Create empty mock repository
      pub fn new() -> Self
      
      /// Create with pre-populated tasks
      pub fn with_tasks(tasks: Vec<Task>) -> Self
      
      /// Create with specific next ID
      pub fn with_next_id(next_id: i32) -> Self
  }
  ```
- [ ] Implement error injection:
  ```rust
  /// Inject error for next operation
  pub fn inject_error(&self, error: TaskError)
  
  /// Clear error injection
  pub fn clear_error(&self)
  ```
- [ ] Implement call tracking:
  ```rust
  /// Get history of called methods
  pub fn call_history(&self) -> Vec<String>
  
  /// Clear call history
  pub fn clear_history(&self)
  
  /// Assert method was called
  pub fn assert_called(&self, method: &str)
  ```

### Task 2: Implement TaskRepository Trait
- [ ] Implement all trait methods with realistic behavior:
  ```rust
  #[async_trait]
  impl TaskRepository for MockTaskRepository {
      async fn create(&self, task: NewTask) -> Result<Task> {
          // Check for error injection
          // Check for duplicate code
          // Create task with next ID
          // Store in HashMap
          // Track call
      }
      
      // ... implement all 8 methods
  }
  ```
- [ ] Ensure state transition validation
- [ ] Simulate realistic delays (optional)
- [ ] Support concurrent access

## Phase 3: Test Data Generators ✓ Required

### Task 3: Create Fixtures (`src/fixtures.rs`)
- [ ] Create standard test tasks:
  ```rust
  /// Create a basic test task
  pub fn create_test_task() -> Task
  
  /// Create task with specific state
  pub fn create_test_task_with_state(state: TaskState) -> Task
  
  /// Create task with specific owner
  pub fn create_test_task_with_owner(owner: &str) -> Task
  
  /// Create multiple unique tasks
  pub fn create_test_tasks(count: usize) -> Vec<Task>
  
  /// Create task in each state
  pub fn create_tasks_in_all_states() -> Vec<Task>
  ```
- [ ] Create NewTask fixtures:
  ```rust
  pub fn create_new_task() -> NewTask
  pub fn create_new_task_with_code(code: &str) -> NewTask
  ```
- [ ] Create UpdateTask fixtures:
  ```rust
  pub fn create_update_task() -> UpdateTask
  pub fn create_update_task_with_name(name: &str) -> UpdateTask
  ```

### Task 4: Create Random Generators (`src/generators.rs`)
- [ ] Use `fake` crate for realistic data:
  ```rust
  /// Generate random task with realistic data
  pub fn generate_random_task() -> Task
  
  /// Generate random task code (e.g., "PROJ-123")
  pub fn generate_task_code() -> String
  
  /// Generate random agent name
  pub fn generate_agent_name() -> String
  
  /// Generate random task name
  pub fn generate_task_name() -> String
  
  /// Generate random task description
  pub fn generate_task_description() -> String
  ```
- [ ] Support custom generators:
  ```rust
  pub struct TaskGenerator {
      code_prefix: String,
      agent_pool: Vec<String>,
  }
  ```

### Task 5: Create Test Builders (`src/builders.rs`)
- [ ] Implement builder pattern for tests:
  ```rust
  pub struct TaskBuilder {
      task: Task,
  }
  
  impl TaskBuilder {
      pub fn new() -> Self
      pub fn with_id(mut self, id: i32) -> Self
      pub fn with_code(mut self, code: impl Into<String>) -> Self
      pub fn with_name(mut self, name: impl Into<String>) -> Self
      pub fn with_state(mut self, state: TaskState) -> Self
      pub fn with_owner(mut self, owner: impl Into<String>) -> Self
      pub fn build(self) -> Task
  }
  ```
- [ ] Create builders for all types:
  ```rust
  pub struct NewTaskBuilder { ... }
  pub struct UpdateTaskBuilder { ... }
  pub struct TaskFilterBuilder { ... }
  ```

## Phase 4: Test Assertions ✓ Required

### Task 6: Create Custom Assertions (`src/assertions.rs`)
- [ ] Task equality assertions:
  ```rust
  /// Assert tasks are equal (ignoring timestamps)
  pub fn assert_task_equals(actual: &Task, expected: &Task)
  
  /// Assert tasks are equal including timestamps
  pub fn assert_task_equals_exact(actual: &Task, expected: &Task)
  
  /// Assert task fields match partially
  pub fn assert_task_matches(task: &Task, matcher: TaskMatcher)
  ```
- [ ] State transition assertions:
  ```rust
  /// Assert state transition is valid
  pub fn assert_state_transition_valid(from: TaskState, to: TaskState)
  
  /// Assert state transition is invalid
  pub fn assert_state_transition_invalid(from: TaskState, to: TaskState)
  ```
- [ ] Collection assertions:
  ```rust
  /// Assert task list contains task with code
  pub fn assert_contains_task_with_code(tasks: &[Task], code: &str)
  
  /// Assert task list is sorted by date
  pub fn assert_tasks_sorted_by_date(tasks: &[Task])
  ```

### Task 7: Create Test Scenarios
- [ ] Common test scenarios:
  ```rust
  pub mod scenarios {
      /// Setup for testing concurrent operations
      pub async fn concurrent_create_scenario() -> (MockTaskRepository, Vec<NewTask>)
      
      /// Setup for testing state transitions
      pub async fn state_transition_scenario() -> (MockTaskRepository, Task)
      
      /// Setup for testing filtering
      pub async fn filtering_scenario() -> (MockTaskRepository, Vec<Task>)
  }
  ```

## Phase 5: Property-Based Testing Support ✓ Required

### Task 8: Create Property Test Generators
- [ ] Proptest strategies:
  ```rust
  use proptest::prelude::*;
  
  /// Strategy for generating valid task codes
  pub fn task_code_strategy() -> impl Strategy<Value = String>
  
  /// Strategy for generating valid task states
  pub fn task_state_strategy() -> impl Strategy<Value = TaskState>
  
  /// Strategy for generating complete tasks
  pub fn task_strategy() -> impl Strategy<Value = Task>
  
  /// Strategy for generating task filters
  pub fn task_filter_strategy() -> impl Strategy<Value = TaskFilter>
  ```

### Task 9: Create Contract Test Helpers
- [ ] Repository contract tests:
  ```rust
  /// Test any TaskRepository implementation
  pub async fn test_repository_contract<R: TaskRepository>(repo: R) {
      test_create_contract(&repo).await;
      test_update_contract(&repo).await;
      test_state_contract(&repo).await;
      // ... all methods
  }
  ```

## Phase 6: Integration Helpers ✓ Required

### Task 10: Create Test Utilities
- [ ] Time manipulation:
  ```rust
  /// Freeze time for testing
  pub struct TimeFreeze { ... }
  
  /// Advance time by duration
  pub fn advance_time(duration: Duration)
  ```
- [ ] Database helpers:
  ```rust
  /// Create in-memory SQLite for testing
  pub async fn create_test_sqlite() -> SqliteTaskRepository
  
  ```

## Public Interface Checklist ✓ MUST MATCH ARCHITECTURE.md

### Repository Mock (`repository.rs`)
- [ ] `MockTaskRepository` struct
- [ ] `new()` constructor
- [ ] `with_tasks(tasks: Vec<Task>)` constructor
- [ ] Implements `core::TaskRepository` trait
- [ ] Error injection methods
- [ ] Call tracking methods

### Fixtures (`fixtures.rs`)
- [ ] `create_test_task()` function
- [ ] `create_test_task_with_state(state: TaskState)` function
- [ ] `create_test_tasks(count: usize)` function

### Assertions (`assertions.rs`)
- [ ] `assert_task_equals(actual: &Task, expected: &Task)`
- [ ] `assert_state_transition_valid(from: TaskState, to: TaskState)`

## Quality Checklist

- [ ] All mocks behave realistically
- [ ] Thread-safe implementation
- [ ] No production dependencies
- [ ] Comprehensive test coverage
- [ ] Clear error messages in assertions
- [ ] Well-documented examples
- [ ] Fast test execution

## Communication Points

Use `./log.sh` to communicate:
```bash
./log.sh "QA-TESTER → ALL: Mock repository ready for use in tests"
./log.sh "QA-TESTER → DATABASE-DESIGNER: Contract tests available for validation"
./log.sh "QA-TESTER → MCP-INTEGRATOR: Test fixtures ready for protocol testing"
```

## Success Criteria

1. All other crates can use mocks for testing
2. Mock behavior matches real implementations
3. Test data is realistic and varied
4. Assertions provide clear failure messages
5. Property tests catch edge cases
6. No flaky tests due to mock issues
7. Fast test execution (<1ms per test)