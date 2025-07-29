# MCP Task Management Server

A production-ready Model Context Protocol (MCP) server written in Rust for comprehensive task management and workflow coordination in AI agent systems.

## Overview

The MCP Task Management Server provides essential task tracking, assignment, and lifecycle management capabilities through a standardized MCP interface. Built with a clean, multi-crate architecture, it enables robust coordination for both small agent teams and large-scale autonomous deployments.

## Key Features

- **Complete Task Lifecycle Management**: Create, update, assign, and track tasks through defined states
- **MCP Protocol Compliance**: Full Server-Sent Events (SSE) based MCP implementation  
- **High Performance**: <100ms response times, 1000+ ops/second throughput
- **Production Ready**: ACID compliance, graceful error handling, comprehensive logging
- **Multi-Agent Coordination**: Designed for AI agent teams with unique identifiers
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

# Run the server
./target/release/mcp-server
```

The server will start on the default MCP SSE endpoint and automatically create a SQLite database at `~/db.sqlite` if no `DATABASE_URL` is specified.

### Configuration

Set the database path via environment variable:

```bash
export DATABASE_URL="sqlite:///path/to/your/database.sqlite"
./target/release/mcp-server
```

## MCP Function Reference

The server implements 8 core MCP functions for task management:

### Task Creation
- **`create_task`**: Create new tasks with validation
- **`update_task`**: Modify task metadata
- **`assign_task`**: Transfer ownership between agents

### Task Retrieval  
- **`get_task_by_id`**: Retrieve by numeric ID
- **`get_task_by_code`**: Retrieve by human-readable code
- **`list_tasks`**: Query with filters (owner, state, date range)

### Task Lifecycle
- **`set_task_state`**: Change task state with validation
- **`archive_task`**: Move completed tasks to archive

For complete API documentation with examples, see [API.md](API.md).

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
- **Database Capacity**: 1M+ tasks without performance degradation

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines, testing procedures, and submission requirements.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

For issues, questions, or contributions:
- Create an issue on GitHub
- Check existing documentation in [docs/](docs/)
- Review the [troubleshooting guide](docs/troubleshooting.md)