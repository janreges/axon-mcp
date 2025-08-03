# Database Library

SQLite implementation of the TaskRepository trait for the MCP Task Management Server, providing production-ready data persistence with migrations, connection pooling, and comprehensive error handling.

## Overview

The `database` crate implements the `TaskRepository` trait defined in `core` using SQLite as the storage backend. It provides:

- **Production-Ready SQLite Backend**: Optimized for high performance and reliability
- **Database Migrations**: Automatic schema management and versioning
- **Connection Pooling**: Efficient resource management for concurrent access
- **Transaction Safety**: ACID compliance with proper error handling
- **Automatic Path Handling**: Defaults to `~/db.sqlite` when no DATABASE_URL is specified

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
database = { path = "../database" }
task-core = { path = "../core" }
```

### Basic Usage

```rust
use database::SqliteTaskRepository;
use task_core::{TaskRepository, NewTask, TaskState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create repository with default database path (~/db.sqlite)
    let repo = SqliteTaskRepository::new().await?;
    
    // Or specify custom database URL
    let repo = SqliteTaskRepository::from_url("sqlite:///path/to/database.sqlite").await?;
    
    // Create a new task
    let new_task = NewTask {
        code: "FEAT-001".to_string(),
        name: "Implement user authentication".to_string(),
        description: "Add JWT-based authentication system".to_string(),
        owner_agent_name: "backend-developer".to_string(),
    };
    
    let task = repo.create(new_task).await?;
    println!("Created task: {} with ID {}", task.code, task.id);
    
    // Update task state
    let updated_task = repo.set_state(task.id, TaskState::InProgress).await?;
    println!("Task {} is now {}", updated_task.code, updated_task.state);
    
    Ok(())
}
```

### Configuration

The repository can be configured via environment variables or programmatically:

```rust
use database::{SqliteTaskRepository, DatabaseConfig};

// Using environment variable
std::env::set_var("DATABASE_URL", "sqlite:///path/to/db.sqlite");
let repo = SqliteTaskRepository::new().await?;

// Using custom configuration
let config = DatabaseConfig::builder()
    .database_url("sqlite:///custom/path.sqlite")
    .max_connections(20)
    .connection_timeout_seconds(30)
    .enable_wal_mode(true)
    .build();

let repo = SqliteTaskRepository::with_config(config).await?;
```

## Features

### Automatic Database Setup

The repository automatically:
- Creates the database file if it doesn't exist
- Runs migrations to set up the schema
- Enables SQLite optimizations (WAL mode, foreign keys, etc.)
- Sets up connection pooling for concurrent access

### Migration System

Migrations are automatically applied on startup:

```sql
-- migrations/sqlite/001_initial.sql
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    owner_agent_name TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('Created', 'InProgress', 'Blocked', 'Review', 'Done', 'Archived')),
    inserted_at TEXT NOT NULL,
    done_at TEXT
);

CREATE INDEX idx_tasks_code ON tasks(code);
CREATE INDEX idx_tasks_owner ON tasks(owner_agent_name);
CREATE INDEX idx_tasks_state ON tasks(state);
CREATE INDEX idx_tasks_inserted_at ON tasks(inserted_at);
```

### Connection Pooling

Efficient connection management with configurable pool size:

```rust
use database::DatabaseConfig;

let config = DatabaseConfig::builder()
    .max_connections(50)           // Maximum concurrent connections  
    .min_connections(5)            // Keep minimum connections alive
    .connection_timeout_seconds(30) // Timeout for getting connection
    .idle_timeout_seconds(600)     // Close idle connections after 10 minutes
    .build();
```

### Error Handling

Comprehensive error mapping from SQLite errors to TaskError types:

```rust
use database::SqliteTaskRepository;
use task_core::TaskError;

let repo = SqliteTaskRepository::new().await?;

match repo.get_by_id(999).await {
    Ok(Some(task)) => println!("Found task: {}", task.name),
    Ok(None) => println!("Task not found"),
    Err(TaskError::Database(msg)) => eprintln!("Database error: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Repository Implementation

The `SqliteTaskRepository` implements all `TaskRepository` methods:

### Task Creation
```rust
async fn create(&self, task: NewTask) -> Result<Task>;
```
- Validates task code uniqueness
- Assigns auto-incrementing ID
- Sets creation timestamp
- Returns complete Task object

### Task Retrieval
```rust
async fn get_by_id(&self, id: i32) -> Result<Option<Task>>;
async fn get_by_code(&self, code: &str) -> Result<Option<Task>>;
async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>>;
```
- Efficient indexed lookups
- Support for complex filtering
- Optional pagination and sorting

### Task Updates
```rust
async fn update(&self, id: i32, updates: UpdateTask) -> Result<Task>;
async fn set_state(&self, id: i32, state: TaskState) -> Result<Task>;
async fn assign(&self, id: i32, new_owner: &str) -> Result<Task>;
```
- Atomic updates with transactions
- State transition validation
- Automatic timestamp management

### Task Archival
```rust
async fn archive(&self, id: i32) -> Result<Task>;
```
- Sets done_at timestamp
- Validates task is in Done state
- Maintains audit trail

### Health and Monitoring
```rust
async fn health_check(&self) -> Result<()>;
async fn get_stats(&self) -> Result<RepositoryStats>;
```
- Connection health verification
- Database statistics for monitoring
- Performance metrics

## Performance Optimizations

### Database Schema
- Optimized indexes on frequently queried columns
- Efficient foreign key constraints
- WAL mode for better concurrent performance

### Query Optimization
- Prepared statements for all operations
- Efficient pagination with LIMIT/OFFSET
- Composite indexes for complex filters

### Connection Management
- Connection pooling reduces connection overhead
- Configurable pool size based on workload
- Automatic connection recycling

### Benchmarks

Typical performance characteristics:
- **Single Task Operations**: Fast response times for individual operations
- **Bulk Operations**: Efficient bulk processing capabilities
- **Concurrent Clients**: Supports multiple simultaneous connections
- **Database Size**: Scales well for typical task management workloads

## Transaction Safety

All operations use appropriate transaction boundaries:

```rust
// Single operations are atomic
let task = repo.create(new_task).await?; // Commits or rolls back

// Multiple operations can be grouped
let tx = repo.begin_transaction().await?;
let task1 = tx.create(new_task1).await?;
let task2 = tx.create(new_task2).await?;
tx.commit().await?; // Both succeed or both fail
```

## Monitoring and Debugging

### Health Checks
```rust
// Verify database connectivity
match repo.health_check().await {
    Ok(()) => println!("Database is healthy"),
    Err(e) => eprintln!("Database health check failed: {}", e),
}
```

### Statistics
```rust
let stats = repo.get_stats().await?;
println!("Total tasks: {}", stats.total_tasks);
println!("Tasks by state: {:?}", stats.tasks_by_state);
println!("Tasks by owner: {:?}", stats.tasks_by_owner);
```

### Debug Logging
Enable SQL query logging:
```bash
RUST_LOG=database=debug cargo run
```

## Configuration Options

### Environment Variables
- `DATABASE_URL`: SQLite database file path (default: `~/db.sqlite`)
- `DATABASE_MAX_CONNECTIONS`: Maximum connection pool size (default: 10)
- `DATABASE_TIMEOUT_SECONDS`: Connection timeout (default: 30)

### Programmatic Configuration
```rust
use database::DatabaseConfig;

let config = DatabaseConfig {
    database_url: "sqlite:///path/to/db.sqlite".to_string(),
    max_connections: 20,
    min_connections: 2,
    connection_timeout: Duration::from_secs(30),
    idle_timeout: Duration::from_secs(600),
    enable_wal_mode: true,
    enable_foreign_keys: true,
    enable_synchronous: false, // For performance in single-writer scenarios
};
```

## Error Handling

The crate maps SQLite errors to appropriate TaskError variants:

| SQLite Error | TaskError | HTTP Status |
|--------------|-----------|-------------|
| UNIQUE constraint | DuplicateCode | 409 |
| NOT NULL constraint | Validation | 400 |
| CHECK constraint | InvalidStateTransition | 422 |
| Connection errors | Database | 500 |
| File I/O errors | Database | 500 |

## Testing

Run the test suite:

```bash
cd database
cargo test
```

### Integration Tests
- Full repository functionality testing
- Concurrent access scenarios
- Error condition handling
- Performance benchmarks

### Contract Tests
Uses the contract tests from the `mocks` crate to ensure compliance:

```rust
use mocks::contract_tests;
use database::SqliteTaskRepository;

#[tokio::test]
async fn test_sqlite_repository_contract() {
    let repo = SqliteTaskRepository::new_in_memory().await.unwrap();
    
    contract_tests::test_all(&repo).await;
}
```

## Migration Management

### Adding New Migrations
1. Create new SQL file in `migrations/sqlite/`
2. Use sequential numbering: `002_add_priority_field.sql`
3. Include both UP and DOWN migration steps
4. Test thoroughly before deployment

### Migration Example
```sql
-- migrations/sqlite/002_add_priority.sql
-- UP
ALTER TABLE tasks ADD COLUMN priority INTEGER NOT NULL DEFAULT 1;
CREATE INDEX idx_tasks_priority ON tasks(priority);

-- DOWN (for rollbacks)
-- DROP INDEX idx_tasks_priority;
-- ALTER TABLE tasks DROP COLUMN priority;
```

## Dependencies

- `sqlx`: Async SQLite driver with compile-time query checking
- `chrono`: Date/time handling with timezone support
- `serde`: JSON serialization for complex types
- `core`: Core domain types and traits
- `tracing`: Structured logging for debugging

## Architecture

The crate is structured for maintainability and testing:

```
database/
├── src/
│   ├── lib.rs          # Public API and configuration
│   ├── repository.rs   # TaskRepository implementation
│   ├── migrations.rs   # Migration management
│   ├── config.rs       # Database configuration
│   └── error.rs        # Error type mapping
├── migrations/
│   └── sqlite/         # SQL migration files
└── tests/
    ├── integration.rs  # Full repository tests
    └── performance.rs  # Benchmark tests
```

## Production Deployment

### Recommended Settings
```rust
let config = DatabaseConfig::builder()
    .database_url("sqlite:///data/tasks.sqlite")
    .max_connections(20)                    // Based on expected load
    .connection_timeout_seconds(5)          // Fail fast
    .idle_timeout_seconds(300)              // Close idle connections
    .enable_wal_mode(true)                  // Better concurrency
    .enable_foreign_keys(true)              // Data integrity
    .enable_synchronous(false)              // Performance over durability
    .build();
```

### Backup Strategy
```bash
# Online backup while server is running
sqlite3 /path/to/tasks.sqlite ".backup /path/to/backup.sqlite"

# Automated backup script
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
sqlite3 /data/tasks.sqlite ".backup /backups/tasks_$DATE.sqlite"
```

### Monitoring
- Monitor connection pool usage
- Track query performance
- Set up health check endpoints
- Log slow queries (>100ms)

## Version

Current version: `0.1.0`