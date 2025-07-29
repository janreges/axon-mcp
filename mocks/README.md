# Mocks Library

Comprehensive testing infrastructure for the MCP Task Management Server, providing mock implementations, test data generators, and testing utilities.

## Overview

The `mocks` crate provides everything needed for thorough testing of the MCP Task Management System:

- **Mock Repository**: Thread-safe implementation of `TaskRepository` with error injection
- **Test Data Generators**: Property-based and realistic test data creation
- **Custom Assertions**: Task-specific assertion helpers for better test readability
- **Test Builders**: Fluent API for building test scenarios
- **Contract Testing**: Standardized tests for trait implementations

## Key Components

### MockTaskRepository
- Thread-safe mock implementation of `TaskRepository`
- Error injection capabilities for failure testing
- Call history tracking for verification
- Realistic behavior simulation

### Test Data Generation
- **Fixtures**: Pre-defined test data for common scenarios
- **Generators**: Property-based random test data creation
- **Builders**: Fluent API for constructing test objects

### Testing Utilities
- **Assertions**: Custom assertions for task-specific validations
- **Contract Tests**: Standardized tests that any repository implementation must pass

## Quick Start

Add to your test dependencies in `Cargo.toml`:

```toml
[dev-dependencies]
mocks = { path = "../mocks" }
```

### Basic Mock Usage

```rust
use mocks::{MockTaskRepository, TaskBuilder, TaskFixtures};
use task_core::{TaskState, NewTask};

#[tokio::test]
async fn test_task_creation() {
    let mock_repo = MockTaskRepository::new();
    
    let new_task = TaskBuilder::new()
        .with_code("TEST-001")
        .with_name("Test Task")
        .with_owner("test-agent")
        .build_new_task();
    
    let created_task = mock_repo.create(new_task).await.unwrap();
    assert_eq!(created_task.state, TaskState::Created);
}
```

### Error Injection Testing

```rust
use mocks::MockTaskRepository;
use task_core::TaskError;

#[tokio::test]
async fn test_database_error_handling() {
    let mock_repo = MockTaskRepository::new();
    
    // Inject database error for next operation
    mock_repo.inject_error(TaskError::Database("Connection failed".to_string()));
    
    let result = mock_repo.get_by_id(1).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().is_database());
}
```

### Test Data Generation

```rust
use mocks::{TaskGenerator, task_fixtures};

#[tokio::test]
async fn test_with_generated_data() {
    let generator = TaskGenerator::new();
    
    // Generate random but valid tasks
    let tasks: Vec<Task> = generator.generate_tasks(10);
    
    // Use pre-defined fixtures
    let completed_task = task_fixtures::completed_task();
    let blocked_task = task_fixtures::blocked_task();
    
    // Test with variety of data
    for task in tasks {
        // Your test logic here
    }
}
```

### Custom Assertions

```rust
use mocks::assert_task_valid_transition;
use task_core::TaskState;

#[tokio::test]
async fn test_state_transitions() {
    let task = create_test_task_in_progress();
    
    // Custom assertions provide better error messages
    assert_task_valid_transition(&task, TaskState::Review);
    assert_task_invalid_transition(&task, TaskState::Archived);
    
    // Check task state progression
    assert_task_can_progress(&task);
}
```

### Fluent Test Builders

```rust
use mocks::TaskBuilder;

#[tokio::test]
async fn test_complex_scenario() {
    let task = TaskBuilder::new()
        .with_code("COMPLEX-001")
        .with_name("Complex Task")
        .with_description("A complex test scenario")
        .with_owner("backend-developer")
        .with_state(TaskState::InProgress)
        .with_created_date_days_ago(5)
        .build();
    
    // Test with the constructed task
    assert_eq!(task.state, TaskState::InProgress);
    assert_eq!(task.owner_agent_name, "backend-developer");
}
```

## Mock Repository Features

### Basic Operations
- **create()**: Creates tasks with auto-incrementing IDs
- **get_by_id() / get_by_code()**: Retrieves tasks with proper not-found handling
- **update()**: Modifies task fields with validation
- **list()**: Filters tasks based on criteria

### Advanced Features
- **Error Injection**: Simulate database failures, validation errors, etc.
- **Call Tracking**: Verify which methods were called and with what parameters
- **State Persistence**: Maintains task state across operations
- **Concurrent Safety**: Thread-safe for parallel test execution

### Configuration Options

```rust
use mocks::MockTaskRepository;

// Empty repository
let repo = MockTaskRepository::new();

// Pre-populated with test data
let repo = MockTaskRepository::with_tasks(vec![
    task_fixtures::created_task(),
    task_fixtures::in_progress_task(),
    task_fixtures::completed_task(),
]);

// Custom starting ID for predictable test IDs
let repo = MockTaskRepository::with_next_id(1000);
```

## Test Data Generators

### Property-Based Testing

```rust
use mocks::TaskGenerator;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_task_codes_are_unique(tasks in TaskGenerator::tasks_strategy(1..100)) {
        let codes: HashSet<_> = tasks.iter().map(|t| &t.code).collect();
        assert_eq!(codes.len(), tasks.len());
    }
}
```

### Realistic Data Generation

```rust
use mocks::generators::*;

// Generate realistic task codes
let code = generate_task_code("ARCH"); // "ARCH-001", "ARCH-002", etc.

// Generate realistic agent names
let agent = generate_agent_name(); // "rust-architect", "database-engineer", etc.

// Generate realistic task descriptions
let description = generate_task_description("authentication");
// "Implement JWT-based authentication with role-based access control"
```

## Contract Testing

Use the provided contract tests to verify any `TaskRepository` implementation:

```rust
use mocks::contract_tests;

#[tokio::test]
async fn test_my_repository_implementation() {
    let repo = MyCustomRepository::new();
    
    // Run all standard contract tests
    contract_tests::test_create_task(&repo).await;
    contract_tests::test_get_by_id(&repo).await;
    contract_tests::test_state_transitions(&repo).await;
    contract_tests::test_concurrent_access(&repo).await;
    
    // All implementations must pass these tests
}
```

## Testing Patterns

### Setup and Teardown

```rust
use mocks::{MockTaskRepository, TestContext};

struct TestSetup {
    repo: MockTaskRepository,
    context: TestContext,
}

impl TestSetup {
    async fn new() -> Self {
        let repo = MockTaskRepository::new();
        let context = TestContext::with_standard_tasks();
        
        Self { repo, context }
    }
    
    async fn create_standard_tasks(&self) {
        for task in &self.context.standard_tasks {
            self.repo.create(task.clone()).await.unwrap();
        }
    }
}
```

### Error Scenario Testing

```rust
use mocks::{MockTaskRepository, error_scenarios};

#[tokio::test]
async fn test_all_error_scenarios() {
    let repo = MockTaskRepository::new();
    
    for scenario in error_scenarios::all() {
        repo.inject_error(scenario.error.clone());
        
        let result = scenario.execute(&repo).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), scenario.expected_status);
    }
}
```

## Performance Testing

```rust
use mocks::{MockTaskRepository, performance_data};
use std::time::Instant;

#[tokio::test]
async fn test_repository_performance() {
    let repo = MockTaskRepository::with_tasks(
        performance_data::large_task_set(10_000)
    );
    
    let start = Instant::now();
    let results = repo.list(TaskFilter::default()).await.unwrap();
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 100); // Should be fast
    assert_eq!(results.len(), 10_000);
}
```

## Architecture

The mocks crate is organized into focused modules:

- **repository.rs**: Mock repository implementation
- **fixtures.rs**: Pre-defined test data
- **generators.rs**: Random data generation
- **builders.rs**: Fluent test object construction
- **assertions.rs**: Custom testing assertions

All components work together to provide comprehensive testing infrastructure that makes writing thorough tests both easy and maintainable.

## Dependencies

- `core`: Core types and traits being mocked
- `async-trait`: Async trait implementation
- `parking_lot`: High-performance synchronization primitives
- `proptest`: Property-based testing framework
- `chrono`: Date/time manipulation for test data

## Version

Current version: `0.1.0`

## Best Practices

1. **Use Fixtures First**: Start with pre-defined fixtures before generating random data
2. **Inject Errors Sparingly**: Only inject errors when testing error handling paths
3. **Verify Call History**: Use call tracking to ensure proper method invocation
4. **Test Edge Cases**: Use generators to find unexpected input combinations
5. **Run Contract Tests**: Ensure all repository implementations pass standard tests