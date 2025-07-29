# MCP Task Server - STDIO Transport Mode

The MCP Task Server supports both HTTP and STDIO transport modes for Model Context Protocol (MCP) communication.

## STDIO Mode Overview

STDIO mode enables the MCP server to run as a subprocess that communicates via stdin/stdout using JSON-RPC 2.0 protocol. This is useful for:

- Integration with MCP-compatible tools and clients
- Running as a subprocess in other applications
- Direct command-line interaction with JSON messages

## Usage

### Starting the Server in STDIO Mode

```bash
# Start server in STDIO mode (default is HTTP mode)
./mcp-server --transport stdio

# Or with explicit configuration
./mcp-server --transport stdio --database-url sqlite:///path/to/db.sqlite
```

### MCP Protocol Handshake

The MCP protocol requires a specific initialization handshake:

1. **Client sends `initialize` request**
2. **Server responds with capabilities**
3. **Client sends `notifications/initialized` notification**
4. **Server is ready to process tool calls**

### Example Communication

#### 1. Initialize Request
```json
{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "test-client", "version": "1.0.0"}}, "id": 1}
```

#### 2. Server Response
```json
{
  "id": 1,
  "jsonrpc": "2.0",
  "result": {
    "capabilities": {
      "tools": {
        "create_task": {
          "description": "Create a new task",
          "inputSchema": {
            "type": "object",
            "properties": {
              "code": {"type": "string"},
              "name": {"type": "string"},
              "description": {"type": "string"},
              "owner_agent_name": {"type": "string"}
            },
            "required": ["code", "name", "description", "owner_agent_name"]
          }
        },
        // ... other tools
      }
    },
    "protocolVersion": "2024-11-05",
    "serverInfo": {
      "name": "mcp-task-server",
      "version": "0.1.0"
    }
  }
}
```

#### 3. Initialized Notification
```json
{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}
```

#### 4. Tool Call Example
```json
{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "create_task", "arguments": {"code": "TASK-001", "name": "Example Task", "description": "An example task", "owner_agent_name": "user"}}, "id": 2}
```

#### 5. Tool Response
```json
{
  "id": 2,
  "jsonrpc": "2.0",
  "result": {
    "id": 1,
    "code": "TASK-001",
    "name": "Example Task",
    "description": "An example task",
    "owner_agent_name": "user",
    "state": "Created",
    "inserted_at": "2025-07-29T21:00:00Z",
    "done_at": null
  }
}
```

## Available Tools

The server exposes these MCP tools:

### Task Management
- `create_task` - Create a new task
- `update_task` - Update an existing task
- `set_task_state` - Change task state (Created, InProgress, Blocked, Review, Done, Archived)
- `assign_task` - Assign task to a different agent
- `archive_task` - Archive a completed task

### Task Querying
- `get_task_by_id` - Retrieve task by numeric ID
- `get_task_by_code` - Retrieve task by human-readable code
- `list_tasks` - List tasks with optional filtering (owner, state, limit, offset)

### System
- `health_check` - Check server health status

## Command Line Testing

You can test the STDIO mode manually using shell commands:

```bash
# Create a test script
cat > test_mcp_stdio.sh << 'EOF'
#!/bin/bash
./mcp-server --transport stdio << 'INPUT'
{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "test-client", "version": "1.0.0"}}, "id": 1}
{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}
{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "list_tasks", "arguments": {}}, "id": 2}
INPUT
EOF

chmod +x test_mcp_stdio.sh
./test_mcp_stdio.sh
```

## Integration with MCP Clients

The STDIO mode is compatible with standard MCP clients. Example configuration for an MCP client:

```json
{
  "mcpServers": {
    "task-manager": {
      "command": "/path/to/mcp-server",
      "args": ["--transport", "stdio"],
      "env": {
        "DATABASE_URL": "sqlite:///path/to/tasks.db"
      }
    }
  }
}
```

## Error Handling

The server properly handles MCP protocol errors:

- **Protocol violations** - Returns JSON-RPC error with code -32006
- **Invalid methods** - Returns method not found error
- **State machine violations** - Enforces proper initialize → initialized → ready flow
- **Tool execution errors** - Returns specific error codes (e.g., -32003 for duplicate task codes)

## Logging

In STDIO mode, the server logs to stderr to avoid interfering with the JSON-RPC communication on stdout:

```bash
# Redirect logs to file for debugging
./mcp-server --transport stdio 2> server.log
```

## Configuration

All server configuration options work in STDIO mode:

```bash
# Custom database location
./mcp-server --transport stdio --database-url sqlite:///custom/path/tasks.db

# Custom log level
./mcp-server --transport stdio --log-level debug

# Configuration file
./mcp-server --transport stdio --config config.toml
```