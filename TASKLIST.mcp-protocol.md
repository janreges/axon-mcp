# Task List: `mcp-protocol` Crate

**Owner Agent**: mcp-integrator  
**Purpose**: Implement MCP server protocol handling using the Rust MCP SDK, bridging core business logic with MCP clients.

## Critical Requirements

This crate MUST:
- Implement the `ProtocolHandler` trait from `core` EXACTLY as specified
- Use the official Rust MCP SDK with Server-Sent Events (SSE) transport
- Handle all MCP function mappings correctly
- Provide proper JSON-RPC error responses over SSE
- Support concurrent client connections via SSE

## Phase 1: Project Setup ✓ Required

- [ ] Create `mcp-protocol/` directory
- [ ] Create `mcp-protocol/Cargo.toml` with dependencies:
  ```toml
  [package]
  name = "mcp-protocol"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  task-core = { path = "../core" }
  mcp-sdk = { git = "https://github.com/modelcontextprotocol/rust-sdk" }
  serde = { version = "1.0", features = ["derive"] }
  serde_json = "1.0"
  tokio = { version = "1.0", features = ["full"] }
  async-trait = "0.1"
  tracing = "0.1"
  anyhow = "1.0"
  axum = { version = "0.7", features = ["ws"] }
  tokio-stream = "0.1"

  [dev-dependencies]
  tokio-test = "0.4"
  mockall = "0.11"
  ```
- [ ] Create directory structure:
  ```
  mcp-protocol/
  ├── src/
  │   ├── lib.rs
  │   ├── handler.rs
  │   ├── server.rs
  │   ├── serialization.rs
  │   └── error.rs
  └── tests/
  ```

## Phase 2: MCP Protocol Implementation ✓ Required

### Task 1: Create Error Handling (`src/error.rs`)
- [ ] Create MCP error mapping:
  ```rust
  pub fn task_error_to_mcp_error(err: core::TaskError) -> mcp_sdk::Error
  ```
- [ ] Map core errors to JSON-RPC error codes:
  - NotFound → -32001
  - Validation → -32002
  - DuplicateCode → -32003
  - InvalidStateTransition → -32004
  - Database → -32005
  - Protocol → -32006
- [ ] Create helper for error responses

### Task 2: Create Serialization Module (`src/serialization.rs`)
- [ ] Implement task serialization for MCP:
  ```rust
  pub fn serialize_task_for_mcp(task: &Task) -> Value
  ```
- [ ] Implement parameter deserialization:
  ```rust
  pub fn deserialize_mcp_params<T: DeserializeOwned>(params: Value) -> Result<T>
  ```
- [ ] Handle datetime serialization (ISO 8601)
- [ ] Handle optional fields properly
- [ ] Create response builders

### Task 3: Implement Protocol Handler (`src/handler.rs`)
- [ ] Create `McpTaskHandler` struct:
  ```rust
  pub struct McpTaskHandler<R: TaskRepository> {
      repository: Arc<R>,
  }
  ```
- [ ] Implement constructor:
  ```rust
  pub fn new(repository: Arc<R>) -> Self
  ```
- [ ] Implement `core::ProtocolHandler` trait methods:
  - [ ] `create_task()` - deserialize params, call repo, serialize response
  - [ ] `update_task()` - handle partial updates correctly
  - [ ] `set_task_state()` - validate state transitions
  - [ ] `get_task_by_id()` - handle not found as null
  - [ ] `get_task_by_code()` - handle not found as null
  - [ ] `list_tasks()` - handle empty results
  - [ ] `assign_task()` - validate and update
  - [ ] `archive_task()` - check preconditions

### Task 4: Create MCP Server (`src/server.rs`)
- [ ] Create `McpServer` struct:
  ```rust
  pub struct McpServer<R: TaskRepository> {
      handler: McpTaskHandler<R>,
  }
  ```
- [ ] Implement server initialization:
  ```rust
  pub fn new(repository: Arc<R>) -> Self
  ```
- [ ] Implement MCP method routing:
  ```rust
  fn route_method(&self, method: &str, params: Value) -> Result<Value>
  ```
- [ ] Map all 8 MCP functions to handler methods:
  ```rust
  match method {
      "create_task" => self.handler.create_task(params),
      "update_task" => self.handler.update_task(params),
      "set_task_state" => self.handler.set_task_state(params),
      "get_task_by_id" => self.handler.get_task_by_id(params),
      "get_task_by_code" => self.handler.get_task_by_code(params),
      "list_tasks" => self.handler.list_tasks(params),
      "assign_task" => self.handler.assign_task(params),
      "archive_task" => self.handler.archive_task(params),
      _ => Err("Method not found")
  }
  ```
- [ ] Implement SSE server startup:
  ```rust
  pub async fn serve(self, addr: &str) -> Result<()>
  ```
- [ ] Set up SSE endpoint for MCP communication
- [ ] Handle SSE client connections and disconnections

### Task 5: MCP SDK Integration with SSE
- [ ] Configure MCP server with proper metadata
- [ ] Register all function handlers for SSE transport
- [ ] Set up capability negotiations over SSE
- [ ] Configure SSE transport with proper headers
- [ ] Implement graceful shutdown for SSE connections
- [ ] Handle SSE reconnection logic

### Task 6: Create Library Root (`src/lib.rs`)
- [ ] Export public types
- [ ] Re-export necessary MCP SDK types
- [ ] Add module documentation

## Phase 3: Protocol Compliance ✓ Required

### Task 7: MCP Function Specifications
Each function must comply with MCP protocol:

- [ ] **create_task**
  - Params: `{code, name, description, owner_agent_name}`
  - Returns: Complete task object
  - Errors: Validation, DuplicateCode

- [ ] **update_task**
  - Params: `{id, name?, description?}`
  - Returns: Updated task object
  - Errors: NotFound, Validation

- [ ] **set_task_state**
  - Params: `{id, state}`
  - Returns: Updated task object
  - Errors: NotFound, InvalidStateTransition

- [ ] **get_task_by_id**
  - Params: `{id}`
  - Returns: Task object or null
  - Errors: None (null for not found)

- [ ] **get_task_by_code**
  - Params: `{code}`
  - Returns: Task object or null
  - Errors: None (null for not found)

- [ ] **list_tasks**
  - Params: `{owner?, state?, date_from?, date_to?}`
  - Returns: Array of task objects
  - Errors: Validation

- [ ] **assign_task**
  - Params: `{id, new_owner}`
  - Returns: Updated task object
  - Errors: NotFound, Validation

- [ ] **archive_task**
  - Params: `{id}`
  - Returns: Archived task object
  - Errors: NotFound, InvalidStateTransition

## Phase 4: Testing ✓ Required

### Task 8: Create Unit Tests
- [ ] Test parameter deserialization for all functions
- [ ] Test response serialization
- [ ] Test error mapping
- [ ] Test method routing
- [ ] Test invalid method handling

### Task 9: Create Integration Tests
- [ ] Create mock repository for testing
- [ ] Test complete request/response cycle
- [ ] Test concurrent requests
- [ ] Test error scenarios
- [ ] Test protocol compliance

### Task 10: Create Protocol Tests
- [ ] Validate JSON-RPC 2.0 compliance
- [ ] Test batch requests
- [ ] Test notification handling
- [ ] Test protocol version negotiation

## Phase 5: Performance & Reliability ✓ Required

- [ ] Implement request ID tracking
- [ ] Add request/response logging
- [ ] Implement rate limiting hooks
- [ ] Add metrics collection points
- [ ] Optimize JSON serialization
- [ ] Handle large response pagination

## Public Interface Checklist ✓ MUST MATCH ARCHITECTURE.md

### Handler (`handler.rs`)
- [ ] `McpTaskHandler<R>` struct
- [ ] `new(repository: Arc<R>) -> Self` constructor
- [ ] Implements `core::ProtocolHandler` trait

### Server (`server.rs`)
- [ ] `McpServer<R>` struct
- [ ] `new(repository: Arc<R>) -> Self` constructor
- [ ] `serve(self, addr: &str) -> Result<()>` method

### Serialization (`serialization.rs`)
- [ ] `serialize_task_for_mcp(task: &Task) -> Value`
- [ ] `deserialize_mcp_params<T>(params: Value) -> Result<T>`

## Quality Checklist

- [ ] Full MCP specification compliance
- [ ] Proper JSON-RPC 2.0 implementation
- [ ] All errors properly mapped
- [ ] No protocol leaks into business logic
- [ ] Comprehensive request validation
- [ ] Proper async handling
- [ ] Thread-safe implementation

## Communication Points

Use `./log.sh` to communicate:
```bash
./log.sh "MCP-INTEGRATOR → RUST-ARCHITECT: Need protocol handler trait review"
./log.sh "MCP-INTEGRATOR → DATABASE-DESIGNER: Testing with repository implementation"
./log.sh "MCP-INTEGRATOR → QA-TESTER: MCP server ready for integration testing"
```

## Success Criteria

1. Full MCP protocol compliance
2. All 8 functions working correctly
3. Proper error handling and mapping
4. Concurrent client support
5. <100ms response time per request
6. Comprehensive test coverage
7. Can be integrated into mcp-server seamlessly