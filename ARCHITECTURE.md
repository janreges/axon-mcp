# MCP Task Management Server - Architecture Document

## Overview

This document describes the multi-crate architecture for the MCP Task Management Server. The system is designed as a Rust workspace with independent crates that can be developed and tested in parallel by different agents, with clear interface contracts between components.

## Workspace Structure

```
task-manager/
├── Cargo.toml                 # Workspace configuration
├── core/                      # Core domain logic (Agent: rust-architect + backend-developer)
├── database/                  # Database implementations (Agent: database-designer)
├── mcp-protocol/             # MCP protocol handler (Agent: mcp-integrator)
├── mcp-server/               # Main server binary (Agent: git-coordinator)
├── mocks/                    # Test utilities (Agent: qa-tester)
├── tests/                    # Integration tests (Agent: qa-tester)
├── docs/                     # Documentation (Agent: documentation-specialist)
├── .github/workflows/        # CI/CD (Agent: devops-engineer)
└── scripts/                  # Build and deployment scripts (Agent: devops-engineer)
```

## Crate Specifications

### 1. `core` - Core Domain Logic
**Owner Agent**: rust-architect + backend-developer  
**Purpose**: Define domain models, business logic, and trait interfaces that all other crates depend on.

#### Public API

```rust
// models.rs - Core domain models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub owner_agent_name: String,
    pub state: TaskState,
    pub inserted_at: DateTime<Utc>,
    pub done_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Created,
    InProgress,
    Blocked,
    Review,
    Done,
    Archived,
}

// DTOs for create/update operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub code: String,
    pub name: String,
    pub description: String,
    pub owner_agent_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateTask {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskFilter {
    pub owner: Option<String>,
    pub state: Option<TaskState>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
}

// error.rs - Error types
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Task not found: {0}")]
    NotFound(String),
    
    #[error("Invalid state transition: {0} to {1}")]
    InvalidStateTransition(TaskState, TaskState),
    
    #[error("Duplicate task code: {0}")]
    DuplicateCode(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
}

pub type Result<T> = std::result::Result<T, TaskError>;

// repository.rs - Database trait
#[async_trait]
pub trait TaskRepository: Send + Sync {
    /// Create a new task
    async fn create(&self, task: NewTask) -> Result<Task>;
    
    /// Update task metadata
    async fn update(&self, id: i32, updates: UpdateTask) -> Result<Task>;
    
    /// Change task state with validation
    async fn set_state(&self, id: i32, state: TaskState) -> Result<Task>;
    
    /// Get task by numeric ID
    async fn get_by_id(&self, id: i32) -> Result<Option<Task>>;
    
    /// Get task by unique code
    async fn get_by_code(&self, code: &str) -> Result<Option<Task>>;
    
    /// List tasks with optional filtering
    async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>>;
    
    /// Assign task to new owner
    async fn assign(&self, id: i32, new_owner: &str) -> Result<Task>;
    
    /// Archive a completed task
    async fn archive(&self, id: i32) -> Result<Task>;
}

// protocol.rs - MCP protocol handler trait
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// Handle MCP create_task function
    async fn create_task(&self, params: CreateTaskParams) -> Result<Task>;
    
    /// Handle MCP update_task function
    async fn update_task(&self, params: UpdateTaskParams) -> Result<Task>;
    
    /// Handle MCP set_task_state function
    async fn set_task_state(&self, params: SetStateParams) -> Result<Task>;
    
    /// Handle MCP get_task_by_id function
    async fn get_task_by_id(&self, id: i32) -> Result<Option<Task>>;
    
    /// Handle MCP get_task_by_code function
    async fn get_task_by_code(&self, code: &str) -> Result<Option<Task>>;
    
    /// Handle MCP list_tasks function
    async fn list_tasks(&self, params: ListTasksParams) -> Result<Vec<Task>>;
    
    /// Handle MCP assign_task function
    async fn assign_task(&self, params: AssignTaskParams) -> Result<Task>;
    
    /// Handle MCP archive_task function
    async fn archive_task(&self, id: i32) -> Result<Task>;
}

// MCP parameter types
#[derive(Debug, Deserialize)]
pub struct CreateTaskParams {
    pub code: String,
    pub name: String,
    pub description: String,
    pub owner_agent_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskParams {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetStateParams {
    pub id: i32,
    pub state: TaskState,
}

#[derive(Debug, Deserialize)]
pub struct ListTasksParams {
    pub owner: Option<String>,
    pub state: Option<TaskState>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct AssignTaskParams {
    pub id: i32,
    pub new_owner: String,
}

// validation.rs - Business logic validation
impl Task {
    /// Validate state transition
    pub fn can_transition_to(&self, new_state: TaskState) -> bool {
        match (self.state, new_state) {
            (TaskState::Created, TaskState::InProgress) => true,
            (TaskState::InProgress, TaskState::Blocked) => true,
            (TaskState::InProgress, TaskState::Review) => true,
            (TaskState::InProgress, TaskState::Done) => true,
            (TaskState::Blocked, TaskState::InProgress) => true,
            (TaskState::Review, TaskState::InProgress) => true,
            (TaskState::Review, TaskState::Done) => true,
            (TaskState::Done, TaskState::Archived) => true,
            _ => false,
        }
    }
}
```

#### Dependencies
- `serde`: Serialization
- `chrono`: DateTime handling
- `thiserror`: Error derive macro
- `async-trait`: Async trait support

### 2. `database` - Database Implementations
**Owner Agent**: database-designer  
**Purpose**: Provide SQLite and PostgreSQL implementations of the TaskRepository trait.

#### Public API

```rust
// lib.rs - Public exports
pub use sqlite::SqliteTaskRepository;
pub use postgres::PostgresTaskRepository;

// sqlite.rs - SQLite implementation
pub struct SqliteTaskRepository {
    pool: SqlitePool,
}

impl SqliteTaskRepository {
    /// Create new SQLite repository with connection string
    pub async fn new(database_url: &str) -> Result<Self>;
    
    /// Run database migrations
    pub async fn migrate(&self) -> Result<()>;
}

// postgres.rs - PostgreSQL implementation  
pub struct PostgresTaskRepository {
    pool: PgPool,
}

impl PostgresTaskRepository {
    /// Create new PostgreSQL repository with connection string
    pub async fn new(database_url: &str) -> Result<Self>;
    
    /// Run database migrations
    pub async fn migrate(&self) -> Result<()>;
}

// Both implement core::TaskRepository trait
```

#### Database Schema

```sql
-- tasks table (same for both SQLite and PostgreSQL)
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    owner_agent_name VARCHAR(100) NOT NULL,
    state VARCHAR(20) NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    done_at TIMESTAMP NULL
);

CREATE INDEX idx_tasks_code ON tasks(code);
CREATE INDEX idx_tasks_owner ON tasks(owner_agent_name);
CREATE INDEX idx_tasks_state ON tasks(state);
CREATE INDEX idx_tasks_inserted_at ON tasks(inserted_at);
```

#### Dependencies
- `core`: Domain models and traits
- `sqlx`: Database driver with SQLite and PostgreSQL features
- `tokio`: Async runtime

### 3. `mcp-protocol` - MCP Protocol Implementation
**Owner Agent**: mcp-integrator  
**Purpose**: Implement MCP server protocol handling using the Rust MCP SDK with Server-Sent Events (SSE) transport.

#### Public API

```rust
// handler.rs - MCP protocol handler
pub struct McpTaskHandler<R: TaskRepository> {
    repository: Arc<R>,
}

impl<R: TaskRepository> McpTaskHandler<R> {
    /// Create new MCP handler with repository
    pub fn new(repository: Arc<R>) -> Self;
}

// Implements core::ProtocolHandler trait

// server.rs - MCP server setup
pub struct McpServer<R: TaskRepository> {
    handler: McpTaskHandler<R>,
}

impl<R: TaskRepository> McpServer<R> {
    /// Create new MCP server
    pub fn new(repository: Arc<R>) -> Self;
    
    /// Start MCP server with SSE transport on specified address
    pub async fn serve(self, addr: &str) -> Result<()>;
}

// serialization.rs - MCP-specific serialization
pub fn serialize_task_for_mcp(task: &Task) -> Value;
pub fn deserialize_mcp_params<T: DeserializeOwned>(params: Value) -> Result<T>;
```

#### MCP Function Mapping

```rust
// Maps MCP function names to handler methods
match method {
    "create_task" => handler.create_task(params),
    "update_task" => handler.update_task(params),
    "set_task_state" => handler.set_task_state(params),
    "get_task_by_id" => handler.get_task_by_id(params),
    "get_task_by_code" => handler.get_task_by_code(params),
    "list_tasks" => handler.list_tasks(params),
    "assign_task" => handler.assign_task(params),
    "archive_task" => handler.archive_task(params),
}
```

#### Dependencies
- `core`: Domain models and traits
- `mcp-sdk`: Official Rust MCP SDK with SSE support
- `serde_json`: JSON serialization
- `tokio`: Async runtime
- `axum`: Web framework for SSE endpoints

### 4. `mcp-server` - Main Server Binary
**Owner Agent**: git-coordinator  
**Purpose**: Assemble all components into a running MCP server with configuration and dependency injection.

#### Binary Structure

```rust
// main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_env()?;
    
    // Get database URL with default fallback
    let database_url = config.database_url.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("sqlite://{}/db.sqlite", home)
    });
    
    // Initialize repository based on config
    let repository: Arc<dyn TaskRepository> = match config.database_type {
        DatabaseType::Sqlite => {
            Arc::new(SqliteTaskRepository::new(&database_url).await?)
        }
        DatabaseType::Postgres => {
            Arc::new(PostgresTaskRepository::new(&database_url).await?)
        }
    };
    
    // Run migrations
    repository.migrate().await?;
    
    // Create and start MCP server with SSE
    let server = McpServer::new(repository);
    server.serve(&config.listen_addr).await?;
    
    Ok(())
}

// config.rs - Configuration management
#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_type: DatabaseType,
    pub database_url: Option<String>,  // Optional, defaults to ~/db.sqlite
    pub listen_addr: String,
    pub log_level: String,
}

#[derive(Debug, Deserialize)]
pub enum DatabaseType {
    Sqlite,
    Postgres,
}
```

#### Dependencies
- `core`: Domain models
- `database`: Repository implementations
- `mcp-protocol`: MCP server implementation
- `tokio`: Async runtime
- `tracing`: Logging
- `dotenv`: Environment configuration

### 5. `mocks` - Test Utilities (Dev Dependency)
**Owner Agent**: qa-tester  
**Purpose**: Provide mock implementations and test utilities for other crates.

#### Public API

```rust
// repository.rs - Mock repository
pub struct MockTaskRepository {
    tasks: Arc<Mutex<HashMap<i32, Task>>>,
    next_id: Arc<AtomicI32>,
}

impl MockTaskRepository {
    pub fn new() -> Self;
    pub fn with_tasks(tasks: Vec<Task>) -> Self;
}

// Implements core::TaskRepository trait

// fixtures.rs - Test data generators
pub fn create_test_task() -> Task;
pub fn create_test_task_with_state(state: TaskState) -> Task;
pub fn create_test_tasks(count: usize) -> Vec<Task>;

// assertions.rs - Custom test assertions
pub fn assert_task_equals(actual: &Task, expected: &Task);
pub fn assert_state_transition_valid(from: TaskState, to: TaskState);
```

#### Dependencies
- `core`: Domain models and traits
- `tokio`: Async runtime
- `parking_lot`: Synchronization primitives

## Integration Strategy

### Development Workflow

1. **Phase 1: Interface Definition**
   - rust-architect defines all traits in `core`
   - All agents review and agree on interfaces
   - Interfaces are frozen for v0.1

2. **Phase 2: Parallel Implementation**
   - Each agent implements their crate independently
   - Use mock implementations for testing
   - Continuous integration via GitHub Actions

3. **Phase 3: Integration**
   - git-coordinator assembles final server
   - Integration testing with all real components
   - Performance and load testing

### Testing Strategy

#### Unit Tests (per crate)
```bash
cd core && cargo test
cd database && cargo test  
cd mcp-protocol && cargo test
```

#### Integration Tests (workspace level)
```bash
cargo test --workspace
cargo test --test integration
```

#### Contract Tests
Each trait implementation must pass standardized contract tests:
```rust
// tests/contracts/repository_contract.rs
pub fn test_repository_contract<R: TaskRepository>(repo: R) {
    // Test all trait methods with standard scenarios
}
```

### CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        crate: [core, database, mcp-protocol, mcp-server]
    steps:
      - uses: actions/checkout@v3
      - run: cargo test -p ${{ matrix.crate }}
      
  integration:
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --workspace
      
  lint:
    steps:
      - run: cargo clippy -- -D warnings
      - run: cargo fmt -- --check
```

## Communication Protocol

All agents use `./log.sh` for inter-agent communication:

```bash
./log.sh "DATABASE-DESIGNER → RUST-ARCHITECT: Schema ready for review"
./log.sh "MCP-INTEGRATOR → BACKEND: Need error mapping for protocol"
./log.sh "QA-TESTER → ALL: Integration test suite ready"
```

## Success Criteria

1. **Independent Development**: Each crate compiles and tests independently
2. **Interface Stability**: No breaking changes to frozen traits
3. **Test Coverage**: >90% coverage per crate
4. **Integration Success**: All components work together seamlessly
5. **Performance**: Meets PRD performance requirements
6. **Documentation**: Complete rustdoc for all public APIs