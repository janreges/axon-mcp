# MCP Task Management Server

A production-ready Model Context Protocol (MCP) server written in Rust for comprehensive task management and workflow coordination in AI agent systems.

## Overview

The MCP Task Management Server provides essential task tracking, assignment, and lifecycle management capabilities through a standardized MCP interface. Built with a clean, multi-crate architecture, it enables robust coordination for both small agent teams and large-scale autonomous deployments.

## Key Features

### Core Task Management
- **Complete Task Lifecycle Management**: Create, update, assign, and track tasks through defined states
- **Advanced Multi-Agent Coordination**: Task discovery, claiming, and release with capability matching
- **Flexible State Machine**: Validated transitions with specialized states for complex workflows

### Inter-Agent Communication
- **Targeted Messaging System**: Send messages to specific agents within task contexts
- **Advanced Message Filtering**: Filter by sender, recipient, message type, and threading
- **Message Threading**: Reply chains and conversation tracking
- **Flexible Message Types**: Support for handoffs, questions, comments, blockers, and custom types

### Technical Excellence
- **Dual Transport Support**: HTTP/SSE and STDIO modes for flexible MCP integration
- **MCP Protocol Compliance**: Full Server-Sent Events (SSE) and JSON-RPC STDIO implementations  
- **High Performance**: <100ms response times, 1000+ ops/second throughput
- **Production Ready**: ACID compliance, graceful error handling, comprehensive logging
- **SQLite Backend**: Automatic database setup with `~/db.sqlite` default path

## Quick Start

### Prerequisites

- Rust 1.75+ with 2024 edition support
- SQLite 3.x (automatically handled)

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd task-manager

# Build the server
cargo build --release

# Run the server (HTTP mode - default)
./target/release/mcp-server

# Run in STDIO mode for MCP client integration
./target/release/mcp-server --transport stdio
```

The server will start on the default MCP SSE endpoint (HTTP mode) or JSON-RPC stdin/stdout (STDIO mode) and automatically create a SQLite database at `~/db.sqlite` if no `DATABASE_URL` is specified.

### Transport Modes

The server supports two transport modes:

- **HTTP Mode** (default): Web server with REST API and Server-Sent Events for MCP
- **STDIO Mode**: JSON-RPC 2.0 over stdin/stdout for direct MCP client integration

See [STDIO_USAGE.md](STDIO_USAGE.md) for detailed STDIO mode documentation and examples.

### Configuration

Set the database path via environment variable:

```bash
export DATABASE_URL="sqlite:///path/to/your/database.sqlite"
./target/release/mcp-server

# Or for STDIO mode
./target/release/mcp-server --transport stdio --database-url sqlite:///custom/path/tasks.db
```

## MCP Function Reference

The server implements 15 comprehensive MCP functions for task management and inter-agent communication:

### Task Management (Core 8 Functions)
- **`create_task`**: Create new tasks with validation
- **`update_task`**: Modify task metadata  
- **`assign_task`**: Transfer ownership between agents
- **`get_task_by_id`**: Retrieve by numeric ID
- **`get_task_by_code`**: Retrieve by human-readable code
- **`list_tasks`**: Query with filters (owner, state, date range)
- **`set_task_state`**: Change task state with validation
- **`archive_task`**: Move completed tasks to archive

### Advanced Multi-Agent Coordination (5 Functions)
- **`discover_work`**: Find available tasks based on agent capabilities
- **`claim_task`**: Atomically claim tasks for execution
- **`release_task`**: Release claimed tasks back to the pool
- **`start_work_session`**: Begin time tracking for task work
- **`end_work_session`**: Complete work session with productivity metrics

### Inter-Agent Messaging (2 Functions)
- **`create_task_message`**: Send targeted messages between agents within tasks
- **`get_task_messages`**: Retrieve messages with advanced filtering options

#### Message Targeting and Filtering

The messaging system supports sophisticated agent-to-agent communication:

**Message Creation with Targeting:**
```json
{
  "method": "create_task_message",
  "params": {
    "task_code": "TASK-001", 
    "author_agent_name": "frontend-developer",
    "target_agent_name": "backend-developer",  // ← NEW: Target specific agent
    "message_type": "handoff",
    "content": "Component ready, need API endpoint"
  }
}
```

**Advanced Message Filtering:**
```json
{
  "method": "get_task_messages", 
  "params": {
    "task_code": "TASK-001",
    "author_agent_name": "frontend-developer",    // Filter by sender
    "target_agent_name": "backend-developer",     // ← NEW: Filter by intended recipient
    "message_type": "handoff",                    // Filter by message type
    "limit": 10
  }
}
```

**Supported Message Types:**
- `handoff` - Work handoffs between agents
- `comment` - General observations and updates
- `question` - Questions requiring responses
- `solution` - Answers and solutions
- `blocker` - Issues preventing progress
- Custom types as needed by your project

For complete API documentation with examples, see [API.md](API.md).

## Use Cases and Examples

### Multi-Agent Workflow Coordination

**Scenario**: Frontend, Backend, and QA agents collaborating on a payment feature

1. **Frontend Developer** creates handoff for backend:
```json
{
  "method": "create_task_message",
  "params": {
    "task_code": "PAY-001",
    "author_agent_name": "frontend-developer", 
    "target_agent_name": "backend-developer",
    "message_type": "handoff",
    "content": "Payment component complete. State structure: {amount, currency, method}. Need matching API endpoint."
  }
}
```

2. **Backend Developer** reads only their targeted messages:
```json
{
  "method": "get_task_messages",
  "params": {
    "task_code": "PAY-001",
    "target_agent_name": "backend-developer"
  }
}
// Returns: Frontend handoff with component details
```

3. **Backend Developer** completes work and hands off to QA:
```json
{
  "method": "create_task_message", 
  "params": {
    "task_code": "PAY-001",
    "author_agent_name": "backend-developer",
    "target_agent_name": "qa-tester", 
    "message_type": "handoff",
    "content": "API endpoint /api/v1/payments ready. Test with staging data."
  }
}
```

4. **QA Tester** asks clarifying question:
```json
{
  "method": "create_task_message",
  "params": {
    "task_code": "PAY-001",
    "author_agent_name": "qa-tester",
    "target_agent_name": "backend-developer",
    "message_type": "question", 
    "content": "What are the valid amount ranges for testing?"
  }
}
```

### Task Discovery and Claiming

**Scenario**: Agents finding and claiming work based on capabilities

```json
// Agent discovers available work
{
  "method": "discover_work",
  "params": {
    "agent_name": "python-developer",
    "capabilities": ["python", "api", "testing"],
    "max_tasks": 5
  }
}

// Agent claims a specific task
{
  "method": "claim_task",
  "params": {
    "task_id": 42,
    "agent_name": "python-developer"
  }
}
```

### Message Filtering Patterns

```json
// Get all handoffs from frontend to backend
{
  "method": "get_task_messages",
  "params": {
    "task_code": "TASK-001",
    "author_agent_name": "frontend-developer",
    "target_agent_name": "backend-developer", 
    "message_type": "handoff"
  }
}

// Get all questions directed at me
{
  "method": "get_task_messages", 
  "params": {
    "task_code": "TASK-001",
    "target_agent_name": "my-agent-name",
    "message_type": "question"
  }
}

// Get recent general updates (no specific target)
{
  "method": "get_task_messages",
  "params": {
    "task_code": "TASK-001", 
    "message_type": "comment",
    "limit": 10
  }
}
```

## Task States

Tasks progress through a defined lifecycle:

```
Created → InProgress → Review → Done → Archived
    ↓         ↓          ↓       ↓
  Blocked ←---+----------+-------+
```

- **Created**: Initial state for new tasks
- **InProgress**: Actively being worked on  
- **Blocked**: Temporarily blocked by dependencies
- **Review**: Awaiting review or approval
- **Done**: Successfully completed
- **Archived**: Moved to long-term storage

## Architecture

The server uses a multi-crate Rust workspace architecture for parallel development:

```
task-manager/
├── core/          # Domain models and business logic
├── database/           # SQLite repository implementation  
├── mcp-protocol/       # MCP server with SSE transport
├── mcp-server/         # Main binary and configuration
└── mocks/              # Test utilities and fixtures
```

Each crate can be developed and tested independently, with clear interface contracts defined in the core crate.

## Development

### Building from Source

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run with development logging
RUST_LOG=debug cargo run
```

### Running Tests

```bash
# Unit tests
cargo test --lib

# Integration tests  
cargo test --test integration

# With coverage
cargo tarpaulin --out html
```

### Documentation

Generate and view documentation:

```bash
# Build documentation
cargo doc --open

# Check documentation
cargo doc --no-deps --document-private-items
```

## Performance

- **Response Time**: <100ms for single task operations
- **Throughput**: >1000 operations per second  
- **Concurrent Clients**: 100+ simultaneous MCP connections
- **Database Capacity**: 1M+ tasks and messages without performance degradation
- **Message Filtering**: Optimized database indexes for fast targeted message retrieval
- **Multi-Agent Scaling**: Supports hundreds of concurrent agents with targeted communication

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines, testing procedures, and submission requirements.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

For issues, questions, or contributions:
- Create an issue on GitHub
- Check existing documentation in [docs/](docs/)
- Review the [troubleshooting guide](docs/troubleshooting.md)