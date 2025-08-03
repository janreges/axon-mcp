# MCP Task Management Server

## Project Overview

A production-ready Model Context Protocol (MCP) server implemented in Rust for comprehensive task management and inter-agent communication. This server provides essential task tracking, assignment, lifecycle management, and sophisticated messaging capabilities through a clean, multi-crate architecture.

The system includes advanced multi-agent coordination features including task discovery, atomic claiming, work session tracking, and a powerful inter-agent messaging system with targeted communication and advanced filtering.

## Project Structure

The project is organized as a Rust workspace with multiple crates:

```
mcp-task-server/
├── Cargo.toml              # Workspace configuration
├── core/                   # Domain models and trait interfaces
├── database/               # SQLite repository implementation  
├── mcp-protocol/          # MCP server with HTTP/SSE transport
├── mcp-server/            # Main binary executable
├── mocks/                 # Test utilities and mock implementations
├── docs/                  # Technical documentation
├── Makefile               # Build and coordination commands
└── LICENSE                # MIT license
```

## Development Guidelines

**Dependency Management**
- Always use `cargo add` commands to add dependencies
- Never manually edit Cargo.toml files
- This ensures latest compatible versions and proper feature flags

```bash
cargo add serde --features derive        # Add with features
cargo add tokio --features full          # Add tokio with all features
cargo add --dev mockall                  # Add dev dependency
cargo add core --path ../core            # Add workspace dependency
```

## Technical Specifications

### Core Task Data Model
```rust
struct Task {
    id: i32,                    // Auto-increment primary key
    code: String,               // Human-readable identifier (e.g., "ARCH-01", "DB-15")
    name: String,               // Brief task title
    description: String,        // Detailed task requirements
    owner_agent_name: Option<String>, // Assigned agent identifier (None for unassigned)
    state: TaskState,           // Current lifecycle state
    inserted_at: DateTime<Utc>, // Creation timestamp
    done_at: Option<DateTime<Utc>>, // Completion timestamp
    
    // MCP v2 Extensions
    workflow_definition_id: Option<i32>,
    workflow_cursor: Option<String>,
    priority_score: f64,
    parent_task_id: Option<i32>,
    failure_count: i32,
    required_capabilities: Vec<String>,
    estimated_effort: Option<i32>,
    confidence_threshold: f64,
}

enum TaskState {
    Created,
    InProgress,
    Blocked,
    Review,
    Done,
    Archived,
    PendingDecomposition,
    PendingHandoff,
    Quarantined,
    WaitingForDependency,
}
```

### Required MCP Functions (22 Total)

#### Core Task Management (9 Functions)
- **create_task**: Add new task with validation
- **update_task**: Modify task details and metadata
- **set_task_state**: Change task lifecycle state
- **get_task_by_id**: Retrieve task by numeric ID
- **get_task_by_code**: Retrieve task by human-readable code
- **list_tasks**: Query tasks with filtering (owner, state, date range)
- **assign_task**: Transfer task ownership between agents
- **archive_task**: Move task to archived state with audit trail
- **health_check**: Check server health and status

#### Advanced Multi-Agent Coordination (5 Functions)
- **discover_work**: Find available tasks based on agent capabilities
- **claim_task**: Atomically claim tasks for execution
- **release_task**: Release claimed tasks back to the pool
- **start_work_session**: Begin time tracking for task work
- **end_work_session**: Complete work session with productivity metrics

#### Inter-Agent Messaging (2 Functions)
- **create_task_message**: Send targeted messages between agents within tasks
- **get_task_messages**: Retrieve messages with advanced filtering by sender, recipient, type

#### Workspace Setup Automation (6 Functions)
- **get_setup_instructions**: Generate AI workspace setup instructions based on PRD analysis
- **get_agentic_workflow_description**: Analyze PRD and recommend optimal agent roles and workflow
- **register_agent**: Register an AI agent in the workspace
- **get_instructions_for_main_ai_file**: Get instructions for creating main AI coordination file
- **create_main_ai_file**: Create the main AI coordination file (CLAUDE.md, etc.)
- **get_workspace_manifest**: Generate complete workspace manifest for AI automation

### Advanced Inter-Agent Messaging System

The messaging system supports sophisticated agent-to-agent communication within task contexts:

```rust
struct TaskMessage {
    id: i32,                           // Auto-increment primary key
    task_code: String,                 // Task code instead of ID
    author_agent_name: String,         // Agent sending the message
    target_agent_name: Option<String>, // Agent the message is intended for (NEW!)
    message_type: String,              // Flexible message type (handoff, question, etc.)
    content: String,                   // Message content
    reply_to_message_id: Option<i32>,  // For threading conversations
    created_at: DateTime<Utc>,         // Message timestamp
}
```

#### Message Targeting Features
- **Targeted Messages**: Send messages to specific agents using `target_agent_name`
- **General Messages**: Broadcast messages by omitting target (null value)
- **Advanced Filtering**: Filter by sender, recipient, message type, threading
- **Message Threading**: Support for conversation chains with reply_to_message_id

#### Supported Message Types (Flexible String-Based)
- `handoff` - Work handoffs between agents with deliverables
- `comment` - General observations and project updates
- `question` - Questions requiring responses from specific agents
- `solution` - Answers and solutions to previous questions
- `blocker` - Issues preventing progress that need attention
- Custom types as needed by your project

### Technology Stack
- **Framework**: Rust with MCP SDK (https://github.com/modelcontextprotocol/rust-sdk)
- **Database**: SQLite ONLY - with automatic path handling (~/axon-mcp.sqlite default)
- **Transport**: Server-Sent Events (SSE) for MCP communication
- **Serialization**: JSON for MCP protocol compliance
- **Testing**: Comprehensive unit, integration, and contract tests
- **Documentation**: Full API documentation with examples

## 🚨 CRITICAL DATABASE REQUIREMENT 🚨

**THIS PROJECT USES SQLite EXCLUSIVELY - NO EXCEPTIONS**

This is a fundamental architectural decision that MUST be strictly enforced:

- ✅ **ONLY SQLite** is permitted for ALL data persistence needs
- ❌ **NEVER** suggest PostgreSQL, MySQL, MongoDB, or any other database
- ✅ **ALL** features must use SQLite: task storage, workspace contexts, session management, messaging, etc.
- ✅ **JSON blob storage** in SQLite is preferred for complex data structures to minimize schema complexity
- ✅ **Single database file** approach: ~/axon-mcp.sqlite handles all persistence needs
- ❌ **NO** multi-database architectures or external database services

**Why SQLite Only:**
- **Simplicity**: Zero-configuration, single-file database
- **Reliability**: ACID compliance, battle-tested, embedded database
- **Performance**: Excellent for MCP server workloads and agent coordination
- **Deployment**: No external dependencies, works anywhere Rust runs
- **Development**: Easy setup, migration, and debugging

**Implementation Guidelines:**
- Use `sqlx` with SQLite backend for all database operations
- Store complex objects as JSON blobs when appropriate
- Use database migrations for schema evolution
- Implement connection pooling for performance
- All repositories must implement SQLite-based persistence

This architectural constraint is non-negotiable and ensures system simplicity, reliability, and ease of deployment.

## Crate Architecture

### Dependency Graph
```
core (base layer - no dependencies)
  ├── database (depends on core)
  ├── mcp-protocol (depends on core)
  └── mocks (depends on core)
      │
      └── mcp-server (depends on core, database, mcp-protocol)
```

### Crate Descriptions

**`core/`** - Foundation crate
- Domain models, business logic, trait interfaces
- Task struct, TaskState enum, error types
- Repository and protocol handler traits

**`database/`** - Data persistence layer
- SQLite implementation of TaskRepository trait
- Database migrations and schema management
- Connection pooling and error mapping

**`mcp-protocol/`** - Protocol implementation
- MCP server with SSE and HTTP transport
- JSON-RPC message handling
- Protocol handler implementation

**`mcp-server/`** - Main executable
- Binary assembling all components
- Configuration management
- Dependency injection and startup logic

**`mocks/`** - Testing utilities
- Mock implementations for testing
- Test fixtures and generators
- Contract test helpers

## Available Make Commands

The project includes a comprehensive Makefile for build automation and project management. Use `make help` to see all available commands:

### Build Commands
```bash
cargo build --workspace          # Build all crates
cargo test --workspace           # Run all tests
cargo check --workspace          # Check compilation
```

### Status Operations
```bash
make check-status               # Show current project status
make check-deps                 # Check if dependencies are ready
make check-crate CRATE=name     # Check specific crate status
```

### Interface Management
```bash
make interface-add AGENT=name INTERFACE=name FILE=path  # Share interface
make interface-check INTERFACE=name                     # Check if interface exists
```

### Decision Tracking
```bash
make decision AGENT=name SUMMARY='summary' RATIONALE='why' ALTERNATIVES='other options'
```

### Utility Commands
```bash
make validate                   # Validate all status codes
make clean-temps               # Remove temporary files
make help                      # Show all available commands
```

## Testing

The project includes comprehensive testing at multiple levels:

- **Unit Tests**: Each crate has its own test suite
- **Integration Tests**: Cross-crate functionality testing  
- **Contract Tests**: Trait implementation validation
- **Mock Testing**: Fast isolated tests using the mocks crate

Run tests with:
```bash
cargo test --workspace         # All tests
cargo test -p core            # Specific crate tests
cargo test --doc              # Documentation tests
```

## Development Best Practices

**Git Workflow**
- Always check `git status` before committing
- Add files selectively, never use `git add .`
- Write meaningful commit messages following conventional commits
- Clean temporary files before committing

**Code Quality**
- Use `cargo clippy` for linting
- Format code with `cargo fmt`
- Maintain test coverage above 90%
- Document all public APIs with rustdoc

## Documentation

The project documentation is organized in the `docs/` folder:

- **`docs/API.md`** - Complete MCP function reference and examples
- **`docs/ARCHITECTURE.md`** - Multi-crate design and interface contracts
- **`docs/PRD.md`** - Product requirements and specifications
- **`docs/MCP.v2.md`** - MCP v2 protocol implementation details

Each crate also contains its own README.md with specific documentation.

## Current Project Status

The project is fully implemented and functional:

### ✅ Completed Features
- **All 22 MCP Functions**: Core task management, advanced coordination, messaging, and workspace automation
- **Multi-Transport Support**: HTTP/JSON-RPC and legacy SSE support
- **SQLite Database**: Full persistence with migrations and connection pooling
- **Comprehensive Testing**: 64+ tests across all crates with high coverage
- **Production Ready**: Error handling, authentication, graceful shutdown
- **Clean Architecture**: Trait-based design with clear separation of concerns

### 🏗️ Technical Implementation
- **MCP Protocol Compliance**: Full 2025-06-18 specification support
- **Database Schema**: Optimized for performance with proper indexing
- **Authentication**: Token-based auth with scope validation
- **Error Handling**: Comprehensive error mapping and JSON-RPC compliance
- **Performance**: Database-level pagination and efficient querying

### 🧪 Quality Assurance
- **Test Coverage**: >90% across all crates
- **Contract Tests**: Ensures trait implementations meet specifications  
- **Integration Tests**: Full system validation
- **Mock Testing**: Fast isolated testing capabilities

## Key Technical Features

- **SQLite Database**: Automatic path handling (~/axon-mcp.sqlite default)
- **MCP v2 Protocol**: Latest specification with backward compatibility
- **Multi-Agent Coordination**: Task discovery, claiming, work sessions
- **Inter-Agent Messaging**: Targeted communication with threading support
- **Production Deployment**: Docker support, health checks, monitoring

## Usage Examples

### Multi-Agent Development Workflow
```rust
// Frontend agent creates handoff for backend
create_task_message(CreateTaskMessageParams {
    task_code: "PAYMENT-FEATURE-001".to_string(),
    author_agent_name: "frontend-developer".to_string(), 
    target_agent_name: Some("backend-developer".to_string()),
    message_type: "handoff".to_string(),
    content: "Payment component ready. State: {amount, currency, method}. Need API endpoint.".to_string(),
    reply_to_message_id: None,
})

// Backend developer reads only targeted messages
get_task_messages(GetTaskMessagesParams {
    task_code: "PAYMENT-FEATURE-001".to_string(),
    target_agent_name: Some("backend-developer".to_string()),
    author_agent_name: None,
    message_type: None,
    reply_to_message_id: None,
    limit: None,
})

// Backend completes work and hands off to QA
create_task_message(CreateTaskMessageParams {
    task_code: "PAYMENT-FEATURE-001".to_string(),
    author_agent_name: "backend-developer".to_string(),
    target_agent_name: Some("qa-tester".to_string()),
    message_type: "handoff".to_string(), 
    content: "API endpoint /api/v1/payments ready. Test with staging data.".to_string(),
    reply_to_message_id: None,
})
```

### Task Discovery and Claiming
```rust
// Agent discovers work based on capabilities
discover_work(DiscoverWorkParams {
    agent_name: "python-specialist".to_string(),
    capabilities: vec!["python".to_string(), "ml".to_string(), "api".to_string()],
    max_tasks: 5,
})

// Agent claims specific work
claim_task(ClaimTaskParams {
    task_id: 42,
    agent_name: "python-specialist".to_string(),
})
```

### Message Filtering Patterns
```rust
// Get all handoffs directed at me
get_task_messages(GetTaskMessagesParams {
    task_code: "TASK-001".to_string(),
    target_agent_name: Some("my-agent-name".to_string()),
    message_type: Some("handoff".to_string()),
    ..Default::default()
})

// Get conversation thread
get_task_messages(GetTaskMessagesParams {
    task_code: "TASK-001".to_string(), 
    reply_to_message_id: Some(message_id),
    ..Default::default()
})
```

## Getting Started

1. **Build the project**:
   ```bash
   cargo build --workspace
   ```

2. **Run tests**:
   ```bash
   cargo test --workspace
   ```

3. **Start the server**:
   ```bash
   cargo run --bin mcp-server
   ```

4. **Check server health**:
   ```bash
   curl http://localhost:3000/health
   ```

The server will automatically create a SQLite database at `~/axon-mcp.sqlite` and start listening on `http://localhost:3000` with MCP endpoints available.