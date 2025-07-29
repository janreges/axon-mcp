# Task List: `database` Crate

**Owner Agent**: database-designer  
**Purpose**: Provide SQLite implementation of the TaskRepository trait from core.

## Critical Requirements

This crate MUST:
- Implement the `TaskRepository` trait from `core` EXACTLY as specified
- Support SQLite with all required features
- Handle all database operations with proper error mapping
- Provide migration support for SQLite
- Be thoroughly tested with in-memory database

## Phase 1: Project Setup ✓ Required

- [ ] Create `database/` directory
- [ ] Create `database/Cargo.toml` with dependencies:
  ```toml
  [package]
  name = "database"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  task-core = { path = "../core" }
  sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "migrate"] }
  tokio = { version = "1.0", features = ["full"] }
  async-trait = "0.1"
  tracing = "0.1"

  [dev-dependencies]
  tokio-test = "0.4"
  ```
- [ ] Create directory structure:
  ```
  database/
  ├── src/
  │   ├── lib.rs
  │   ├── sqlite.rs
  │   ├── migrations.rs
  │   └── common.rs
  ├── migrations/
  │   └── sqlite/
  └── tests/
  ```

## Phase 2: Database Schema ✓ Required

### Task 1: Create SQLite Migrations
- [ ] Create `migrations/sqlite/001_create_tasks.sql`:
  ```sql
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
  ```
- [ ] Create indices:
  ```sql
  CREATE INDEX idx_tasks_code ON tasks(code);
  CREATE INDEX idx_tasks_owner ON tasks(owner_agent_name);
  CREATE INDEX idx_tasks_state ON tasks(state);
  CREATE INDEX idx_tasks_inserted_at ON tasks(inserted_at);
  ```


## Phase 3: Implementation ✓ Required

### Task 3: Create Common Module (`src/common.rs`)
- [ ] Create shared SQL query builders
- [ ] Create state string conversion helpers:
  ```rust
  pub fn state_to_string(state: TaskState) -> &'static str
  pub fn string_to_state(s: &str) -> Result<TaskState>
  ```
- [ ] Create error conversion from sqlx to core::TaskError
- [ ] Create row mapping utilities

### Task 4: Implement SQLite Repository (`src/sqlite.rs`)
- [ ] Create `SqliteTaskRepository` struct:
  ```rust
  pub struct SqliteTaskRepository {
      pool: SqlitePool,
  }
  ```
- [ ] Implement constructor:
  ```rust
  pub async fn new(database_url: &str) -> Result<Self>
  ```
- [ ] Implement migration method:
  ```rust
  pub async fn migrate(&self) -> Result<()>
  ```
- [ ] Implement ALL methods from `TaskRepository` trait:
  - [ ] `create()` - with RETURNING clause for ID
  - [ ] `update()` - with partial updates
  - [ ] `set_state()` - with state validation
  - [ ] `get_by_id()` - handle not found gracefully
  - [ ] `get_by_code()` - handle not found gracefully
  - [ ] `list()` - with dynamic query building
  - [ ] `assign()` - update owner field
  - [ ] `archive()` - validate state before archiving


### Task 6: Create Library Root (`src/lib.rs`)
- [ ] Export SQLite repository implementation
- [ ] Re-export common types
- [ ] Simple public API:
  ```rust
  pub use sqlite::SqliteTaskRepository;
  ```

## Phase 4: Testing ✓ Required

### Task 7: Create Integration Tests
- [ ] Create `tests/sqlite_integration.rs`:
  - Test with in-memory database (`:memory:`)
  - Test all repository methods
  - Test error conditions
  - Test concurrent operations
- [ ] Create `tests/contract.rs`:
  - Generic tests that both implementations must pass
  - Ensure identical behavior

### Task 8: Create Unit Tests
- [ ] Test state conversions
- [ ] Test error mappings
- [ ] Test query builders
- [ ] Test migration execution

## Phase 5: Performance & Reliability ✓ Required

- [ ] Implement connection pooling with sensible defaults
- [ ] Add retry logic for transient failures
- [ ] Implement proper transaction handling
- [ ] Add query timeouts
- [ ] Optimize indices based on query patterns
- [ ] Add connection health checks

## Public Interface Checklist ✓ MUST MATCH ARCHITECTURE.md

### SQLite Repository (`sqlite.rs`)
- [ ] `SqliteTaskRepository` struct
- [ ] `new(database_url: &str) -> Result<Self>` constructor
- [ ] `migrate(&self) -> Result<()>` method
- [ ] Implements `core::TaskRepository` trait fully


## Error Handling Requirements

- [ ] Map all database errors to appropriate `core::TaskError` variants
- [ ] Handle unique constraint violations → `DuplicateCode`
- [ ] Handle not found → return `Ok(None)` not error
- [ ] Handle connection errors → `Database` error
- [ ] Provide meaningful error messages

## Quality Checklist

- [ ] No SQL injection vulnerabilities (use parameterized queries)
- [ ] All queries use prepared statements
- [ ] Proper index usage for performance
- [ ] Connection pooling configured correctly
- [ ] All timestamps handled correctly
- [ ] No N+1 query problems
- [ ] Transaction boundaries correct

## Communication Points

Use `./log.sh` to communicate:
```bash
./log.sh "DATABASE-DESIGNER → RUST-ARCHITECT: Need clarification on error mapping"
./log.sh "DATABASE-DESIGNER → QA-TESTER: Database implementation ready for testing"
./log.sh "DATABASE-DESIGNER → MCP-INTEGRATOR: Repository implementation complete"
```

## Success Criteria

1. SQLite implementation works perfectly
2. All trait methods implemented correctly
3. Comprehensive test coverage (>90%)
4. Performance meets requirements (<100ms operations)
5. Proper error handling and recovery
6. Database migrations work reliably
7. Can be used by mcp-server without issues