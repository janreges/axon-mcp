# Contributing to MCP Task Management Server

Thank you for your interest in contributing to the MCP Task Management Server! This guide will help you get started with development, testing, and submitting contributions.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Code Organization](#code-organization)
- [Development Workflow](#development-workflow)
- [Testing Guidelines](#testing-guidelines)
- [Code Style and Standards](#code-style-and-standards)
- [Pull Request Process](#pull-request-process)
- [Issue Guidelines](#issue-guidelines)

## Getting Started

### Prerequisites

- **Rust**: 1.75+ with 2024 edition support
- **Git**: For version control
- **SQLite**: For database operations (usually pre-installed)
- **Docker**: For containerized testing (optional)

### Fork and Clone

```bash
# Fork the repository on GitHub
# Then clone your fork
git clone https://github.com/your-username/mcp-task-server.git
cd mcp-task-server

# Add upstream remote
git remote add upstream https://github.com/original-org/mcp-task-server.git
```

### Initial Setup

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run the server
cargo run

# Check formatting and linting
cargo fmt --check
cargo clippy -- -D warnings
```

## Development Environment

### Recommended Tools

- **IDE**: VS Code with rust-analyzer extension, or IntelliJ IDEA with Rust plugin
- **Database**: SQLite browser for database inspection
- **HTTP Client**: curl, Postman, or HTTP client extension for testing APIs
- **Git UI**: Optional - GitKraken, Sourcetree, or VS Code Git integration

### Environment Setup

```bash
# Install development dependencies
cargo install cargo-tarpaulin  # Code coverage
cargo install cargo-audit      # Security auditing
cargo install cargo-outdated   # Dependency checking

# Set up git hooks (optional)
cp scripts/pre-commit .git/hooks/
chmod +x .git/hooks/pre-commit
```

### Configuration for Development

Create `config/development.toml`:

```toml
[server]
listen_addr = "127.0.0.1:3001"
timeout_seconds = 5

[database]
url = "sqlite:///dev.sqlite"
max_connections = 2

[logging]
level = "debug"
format = "pretty"
enable_colors = true
```

## Code Organization

### Workspace Structure

The project uses a Rust workspace with multiple crates:

```
task-manager/
├── core/           # Domain models and business logic
├── database/       # SQLite repository implementation
├── mcp-protocol/   # MCP server with SSE transport
├── mcp-server/     # Main binary
├── mocks/          # Test utilities and fixtures
└── docs/           # Additional documentation
```

### Crate Dependencies

```
core (no dependencies - base layer)
  ├── database (depends on core)
  ├── mcp-protocol (depends on core)
  └── mocks (depends on core)
      │
      └── mcp-server (depends on core, database, mcp-protocol)
```

### Key Design Principles

1. **Separation of Concerns**: Each crate has a specific responsibility
2. **Trait-Based Design**: Use traits for abstraction and testability
3. **Async by Default**: All I/O operations are async
4. **Error Handling**: Comprehensive error types with context
5. **Testing**: High test coverage with unit, integration, and contract tests

## Development Workflow

### Branch Naming

Use descriptive branch names:

```bash
# Feature branches
git checkout -b feature/add-task-priorities

# Bug fixes
git checkout -b fix/database-connection-leak

# Documentation
git checkout -b docs/api-examples

# Refactoring
git checkout -b refactor/simplify-error-handling
```

### Commit Messages

Follow conventional commit format:

```
type(scope): brief description

Longer description if needed, explaining what and why.

Fixes #123
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(core): add task priority field to Task model

Add priority field (1-5 scale) to enable task prioritization.
Includes database migration and API updates.

Fixes #45

fix(database): prevent connection pool exhaustion

Ensure connections are properly returned to pool on error.
Adds connection timeout and better error handling.

Fixes #78
```

### Development Process

1. **Create Issue**: Discuss significant changes before implementing
2. **Create Branch**: Use descriptive branch names
3. **Implement Changes**: Follow code style and testing guidelines
4. **Write Tests**: Ensure good test coverage
5. **Update Documentation**: Update relevant docs and comments
6. **Submit PR**: Create pull request with clear description

### Keeping Your Fork Updated

```bash
# Fetch upstream changes
git fetch upstream

# Merge upstream changes into main
git checkout main
git merge upstream/main

# Rebase your feature branch
git checkout feature/your-feature
git rebase main
```

## Testing Guidelines

### Test Organization

- **Unit Tests**: Test individual functions and methods
- **Integration Tests**: Test crate interactions
- **Contract Tests**: Ensure trait implementations comply with contracts
- **End-to-End Tests**: Test complete workflows

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p core

# Run specific test
cargo test test_task_creation

# Run tests with output
cargo test -- --nocapture

# Run ignored tests (performance, etc.)
cargo test -- --ignored
```

### Test Coverage

```bash
# Generate coverage report
cargo tarpaulin --out html

# View coverage
open tarpaulin-report.html
```

### Writing Tests

#### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_state_transitions() {
        let task = Task {
            state: TaskState::Created,
            // ... other fields
        };
        
        assert!(task.can_transition_to(TaskState::InProgress));
        assert!(!task.can_transition_to(TaskState::Done));
    }

    #[tokio::test]
    async fn test_repository_create() {
        let repo = MockTaskRepository::new();
        let new_task = NewTask {
            code: "TEST-001".to_string(),
            // ... other fields
        };
        
        let result = repo.create(new_task).await;
        assert!(result.is_ok());
    }
}
```

#### Integration Tests

```rust
// tests/integration_tests.rs
use mcp_server::{create_repository, create_server};
use task_core::NewTask;

#[tokio::test]
async fn test_full_workflow() {
    let repo = create_repository(":memory:").await.unwrap();
    let server = create_server(repo);
    
    // Test complete task lifecycle
    let task = create_test_task(&server).await;
    let updated = update_task_state(&server, task.id, TaskState::InProgress).await;
    
    assert_eq!(updated.state, TaskState::InProgress);
}
```

#### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_task_code_uniqueness(
        codes in prop::collection::vec(
            "[A-Z]{2,4}-[0-9]{1,3}",
            1..100
        )
    ) {
        let unique_codes: HashSet<_> = codes.iter().collect();
        prop_assert_eq!(unique_codes.len(), codes.len());
    }
}
```

### Performance Tests

```rust
#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_concurrent_task_creation() {
    let repo = SqliteTaskRepository::new(":memory:").await.unwrap();
    
    let tasks: Vec<_> = (0..1000)
        .map(|i| create_test_task(i))
        .collect();
    
    let start = Instant::now();
    let futures: Vec<_> = tasks.into_iter()
        .map(|task| repo.create(task))
        .collect();
    
    let results = future::join_all(futures).await;
    let duration = start.elapsed();
    
    assert!(results.iter().all(|r| r.is_ok()));
    assert!(duration < Duration::from_millis(1000));
}
```

## Code Style and Standards

### Rust Style Guidelines

Follow standard Rust conventions:

```bash
# Format code
cargo fmt

# Check clippy lints
cargo clippy -- -D warnings

# Check for unused dependencies
cargo machete
```

### Code Quality Standards

1. **Error Handling**: Use proper error types, avoid unwrap() in production code
2. **Documentation**: All public APIs must have rustdoc comments
3. **Testing**: Maintain >90% test coverage
4. **Performance**: Consider performance implications of changes
5. **Security**: Follow security best practices

### Documentation Standards

#### Rustdoc Comments

```rust
/// Creates a new task in the system.
///
/// # Arguments
///
/// * `task` - The new task data to create
///
/// # Returns
///
/// Returns the created task with assigned ID and timestamps.
///
/// # Errors
///
/// This function will return an error if:
/// - The task code already exists (`TaskError::DuplicateCode`)
/// - The task data is invalid (`TaskError::Validation`)
/// - The database operation fails (`TaskError::Database`)
///
/// # Examples
///
/// ```rust
/// use task_core::{NewTask, TaskRepository};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let new_task = NewTask {
///     code: "FEAT-001".to_string(),
///     name: "Add new feature".to_string(),
///     description: "Implement user-requested feature".to_string(),
///     owner_agent_name: "developer".to_string(),
/// };
///
/// let task = repository.create(new_task).await?;
/// println!("Created task: {}", task.id);
/// # Ok(())
/// # }
/// ```
pub async fn create(&self, task: NewTask) -> Result<Task>;
```

#### Code Comments

```rust
// Good: Explain why, not what
// Use exponential backoff to handle transient database errors
let mut retry_delay = Duration::from_millis(100);

// Bad: Explain what is obvious
// Increment the counter by 1
counter += 1;
```

### Error Handling

#### Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Task not found: {0}")]
    NotFound(String),
    
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: TaskState, to: TaskState },
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

#### Error Context

```rust
use anyhow::{Context, Result};

fn load_config(path: &str) -> Result<Config> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path))?
        .parse()
        .with_context(|| "Failed to parse config file")
}
```

## Pull Request Process

### Before Submitting

1. **Run Tests**: Ensure all tests pass
2. **Check Coverage**: Maintain or improve test coverage
3. **Update Documentation**: Update relevant docs and comments
4. **Check Formatting**: Run `cargo fmt` and `cargo clippy`
5. **Test Locally**: Test your changes thoroughly

### PR Checklist

- [ ] Tests pass locally (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated (if applicable)
- [ ] CHANGELOG.md updated (for user-facing changes)
- [ ] Tests added/updated for new functionality
- [ ] Performance impact considered
- [ ] Security implications reviewed

### PR Description Template

```markdown
## Description
Brief description of changes made.

## Type of Change
- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed

## Performance Impact
Describe any performance implications.

## Security Considerations
Describe any security implications.

## Related Issues
Fixes #123
Related to #456
```

### Review Process

1. **Automated Checks**: CI will run tests, linting, and security checks
2. **Code Review**: Maintainers will review code quality and design
3. **Discussion**: Address feedback and make requested changes
4. **Approval**: Once approved, PR will be merged

### After Merge

1. **Update Local Repo**: Pull latest changes
2. **Delete Branch**: Clean up feature branch
3. **Monitor**: Watch for any issues after deployment

## Issue Guidelines

### Bug Reports

Use the bug report template:

```markdown
**Bug Description**
A clear description of the bug.

**Steps to Reproduce**
1. Go to '...'
2. Click on '....'
3. See error

**Expected Behavior**
What you expected to happen.

**Actual Behavior**
What actually happened.

**Environment**
- OS: [e.g. Ubuntu 20.04]
- Rust version: [e.g. 1.75.0]
- Server version: [e.g. 0.1.0]

**Additional Context**
Add any other context about the problem here.
```

### Feature Requests

Use the feature request template:

```markdown
**Feature Description**
A clear description of the feature you'd like to see.

**Use Case**
Explain the problem this feature would solve.

**Proposed Solution**
Describe how you envision this feature working.

**Alternatives Considered**
Any alternative solutions you've considered.

**Additional Context**
Any other context or screenshots about the feature request.
```

### Question/Discussion

For questions or discussions, use the appropriate labels and provide:

- Clear description of what you're asking
- Context about what you're trying to achieve
- What you've already tried
- Relevant code snippets or configuration

## Development Best Practices

### Performance Considerations

1. **Database Queries**: Use efficient queries with proper indexes
2. **Memory Usage**: Avoid unnecessary allocations
3. **Async Operations**: Don't block the async runtime
4. **Connection Pooling**: Reuse database connections

### Security Practices

1. **Input Validation**: Validate all inputs
2. **SQL Injection**: Use parameterized queries
3. **Error Messages**: Don't leak sensitive information
4. **Dependencies**: Keep dependencies updated

### Debugging Tips

```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Enable specific module logging
RUST_LOG=database=trace,mcp_protocol=debug cargo run

# Profile performance
cargo build --release
perf record target/release/mcp-server
perf report
```

## Release Process

### Version Numbering

We follow Semantic Versioning (SemVer):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist

1. Update version in `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create release tag
5. Build release artifacts
6. Update documentation

## Getting Help

- **Discord/Slack**: Join our development chat
- **GitHub Discussions**: For general questions
- **GitHub Issues**: For bug reports and feature requests
- **Documentation**: Check existing docs first

## Recognition

Contributors are recognized in:

- `CONTRIBUTORS.md` file
- Release notes
- Project documentation

Thank you for contributing to the MCP Task Management Server!