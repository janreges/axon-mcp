# MCP Protocol Library

High-performance Model Context Protocol (MCP) server implementation using Server-Sent Events (SSE) transport for the Task Management Server.

## Overview

The `mcp-protocol` crate provides the MCP server implementation that bridges between core task management business logic and MCP clients. It offers:

- **JSON-RPC 2.0 Compliance**: Full protocol adherence with proper error handling
- **SSE Transport**: Real-time Server-Sent Events communication
- **High Performance**: <1ms response times with concurrent client support
- **Error Mapping**: Comprehensive error translation from core types to MCP responses
- **Type Safety**: Compile-time validation of MCP message formats

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
mcp-protocol = { path = "../mcp-protocol" }
task-core = { path = "../core" }
database = { path = "../database" }
```

### Basic Server Setup

```rust
use mcp_protocol::McpServer;
use database::SqliteTaskRepository;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create repository
    let repository = Arc::new(SqliteTaskRepository::new().await?);
    
    // Create MCP server
    let server = McpServer::new(repository);
    
    // Start server on default MCP port
    server.serve("127.0.0.1:3000").await?;
    
    Ok(())
}
```

### Client Connection

Connect to the MCP server using SSE:

```javascript
// Establish SSE connection
const eventSource = new EventSource('http://localhost:3000/mcp/v1');

eventSource.onmessage = function(event) {
    const response = JSON.parse(event.data);
    console.log('MCP Response:', response);
};

// Send MCP request
fetch('http://localhost:3000/mcp/v1/rpc', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
        jsonrpc: "2.0",
        id: "req-001",
        method: "create_task",
        params: {
            code: "FEAT-001",
            name: "Implement feature",
            description: "Add new functionality",
            owner_agent_name: "developer"
        }
    })
});
```

## Protocol Implementation

### MCP Functions

The server implements all 8 MCP task management functions:

- **create_task**: Create new tasks with validation
- **update_task**: Modify task metadata  
- **set_task_state**: Change task lifecycle state
- **get_task_by_id**: Retrieve task by numeric ID
- **get_task_by_code**: Retrieve task by human-readable code
- **list_tasks**: Query tasks with filtering
- **assign_task**: Transfer task ownership
- **archive_task**: Move tasks to archived state

### Request Format

All requests follow JSON-RPC 2.0 specification:

```json
{
    "jsonrpc": "2.0",
    "id": "unique-request-id",
    "method": "function_name", 
    "params": {
        "parameter": "value"
    }
}
```

### Response Format

Successful responses:

```json
{
    "jsonrpc": "2.0",
    "id": "unique-request-id",
    "result": {
        "task_data": "here"
    }
}
```

Error responses:

```json
{
    "jsonrpc": "2.0",
    "id": "unique-request-id", 
    "error": {
        "code": -32000,
        "message": "Task not found",
        "data": {
            "task_id": 123
        }
    }
}
```

## Server Architecture

### Components

- **McpServer**: Main server managing HTTP/SSE endpoints
- **McpTaskHandler**: Protocol handler implementing all MCP functions
- **Error Mapping**: Translation between core and MCP error types
- **Serialization**: JSON conversion for MCP messages

### Transport Layer

The server provides two endpoints:

- **`/mcp/v1`** (GET): SSE endpoint for receiving responses
- **`/mcp/v1/rpc`** (POST): JSON-RPC endpoint for sending requests

### Concurrent Handling

- **Thread-Safe**: All operations support concurrent access
- **Connection Pooling**: Efficient management of multiple clients
- **Async Processing**: Non-blocking request handling
- **Resource Management**: Automatic cleanup of disconnected clients

## Error Handling

### MCP Error Codes

The server maps core errors to standard MCP error codes:

| Core Error | MCP Code | Description |
|------------|----------|-------------|
| TaskError::NotFound | -32000 | Task not found |
| TaskError::DuplicateCode | -32001 | Task code already exists |
| TaskError::InvalidStateTransition | -32002 | Invalid state change |
| TaskError::Validation | -32003 | Parameter validation failed |
| TaskError::Database | -32004 | Database operation failed |
| Internal errors | -32005 | Protocol processing error |

### Error Context

Errors include contextual information:

```json
{
    "error": {
        "code": -32002,
        "message": "Invalid state transition",
        "data": {
            "current_state": "Created",
            "attempted_state": "Done",
            "valid_transitions": ["InProgress", "Blocked"]
        }
    }
}
```

## Performance Characteristics

### Response Times
- **Single Task Operations**: Fast response times for individual operations
- **List Operations**: Efficient querying with database-level optimization
- **Concurrent Requests**: Scales well with multiple simultaneous clients

### Throughput
- **Operations**: High throughput suitable for multi-agent coordination
- **Sustained Load**: Consistent performance under typical workloads
- **Memory Usage**: Efficient memory management for concurrent operations

### Benchmarks

Run performance tests:

```bash
cd mcp-protocol
cargo test --release performance
```

Performance tests validate operational efficiency and concurrent handling capabilities.

## Configuration

### Server Configuration

```rust
use mcp_protocol::{McpServer, ServerConfig};

let config = ServerConfig {
    listen_addr: "0.0.0.0:3000".to_string(),
    max_connections: 1000,
    request_timeout_ms: 30000,
    keepalive_interval_ms: 30000,
    enable_cors: true,
};

let server = McpServer::with_config(repository, config);
```

### Transport Options

```rust
use mcp_protocol::TransportConfig;

let transport = TransportConfig {
    sse_heartbeat_interval: Duration::from_secs(30),
    max_message_size: 1024 * 1024, // 1MB
    compression_enabled: true,
    keep_alive_timeout: Duration::from_secs(60),
};
```

## Testing

### Integration Tests

```rust
use mcp_protocol::{McpServer, test_utils};
use mocks::MockTaskRepository;

#[tokio::test]
async fn test_create_task_flow() {
    let repo = Arc::new(MockTaskRepository::new());
    let server = McpServer::new(repo);
    
    let client = test_utils::create_test_client(server).await;
    
    let response = client.create_task(CreateTaskParams {
        code: "TEST-001".to_string(),
        name: "Test Task".to_string(),
        description: "Test description".to_string(),
        owner_agent_name: "test-agent".to_string(),
    }).await.unwrap();
    
    assert_eq!(response.state, TaskState::Created);
}
```

### Protocol Compliance Tests

```bash
cd mcp-protocol
cargo test protocol_compliance
```

Verifies:
- JSON-RPC 2.0 format compliance
- Proper error code mapping  
- SSE message formatting
- Request/response correlation

### Performance Tests

```bash
cargo test --release performance -- --nocapture
```

Tests concurrent load, memory usage, and response times.

## Development

### Adding New MCP Functions

1. **Define Parameters**: Add parameter struct in `core/src/protocol.rs`
2. **Implement Handler**: Add method to `McpTaskHandler`
3. **Add Routing**: Register endpoint in `McpServer::create_router()`
4. **Add Tests**: Create integration tests

Example:

```rust
// 1. Add parameters
#[derive(Debug, Serialize, Deserialize)]
pub struct CustomActionParams {
    pub task_id: i32,
    pub action_data: String,
}

// 2. Implement handler
impl<R: TaskRepository> McpTaskHandler<R> {
    pub async fn custom_action(&self, params: CustomActionParams) -> Result<Task> {
        // Implementation here
    }
}

// 3. Register routing in server.rs
router.route("/custom_action", post(custom_action_handler))
```

### Debugging

Enable debug logging:

```bash
RUST_LOG=mcp_protocol=debug cargo run
```

View protocol messages:

```bash
RUST_LOG=mcp_protocol::serialization=trace cargo run
```

## Dependencies

- **axum**: High-performance HTTP server framework
- **tokio**: Async runtime for concurrent processing
- **serde**: JSON serialization/deserialization  
- **serde_json**: JSON processing with error handling
- **tower**: HTTP middleware and utilities
- **tracing**: Structured logging and instrumentation
- **task-core**: Core business logic and types

## Security Considerations

### Input Validation

- All parameters validated before processing
- JSON schema validation for message format
- SQL injection prevention via parameterized queries
- Size limits on incoming messages

### Error Information

- Error messages don't leak sensitive data
- Stack traces excluded from client responses
- Database connection details not exposed
- Internal paths and configurations hidden

### Future Security Features

- Authentication and authorization
- Rate limiting per client
- Request signing and verification
- TLS termination support

## Monitoring

### Health Checks

```rust
// Check server health
let health = server.health_check().await?;
println!("Server status: {:?}", health);
```

### Metrics

The server exposes metrics for monitoring:

- Request count by method
- Response time percentiles  
- Error rate by error type
- Active connection count
- Memory usage statistics

### Logging

Structured logs include:

```json
{
    "timestamp": "2025-01-29T10:30:00Z",
    "level": "INFO", 
    "method": "create_task",
    "request_id": "req-123",
    "duration_ms": 2.3,
    "client_ip": "192.168.1.100",
    "result": "success"
}
```

## Troubleshooting

### Common Issues

**Connection refused**: 
- Check server is running: `ps aux | grep mcp-server`
- Verify port binding: `netstat -tlnp | grep 3000`

**Request timeout**:
- Check database connectivity
- Monitor repository response times
- Review resource utilization

**Invalid JSON-RPC**:
- Validate request format
- Check parameter types
- Review error response details

### Debug Mode

```bash
# Start server with debug logging
RUST_LOG=debug ./mcp-server

# Test specific endpoint
curl -X POST http://localhost:3000/mcp/v1/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":"test","method":"get_task_by_id","params":{"task_id":1}}'
```

## Architecture Decisions

### Why SSE over WebSockets?

- **Simplicity**: Easier client implementation
- **Reliability**: Built-in reconnection handling
- **Firewall Friendly**: Standard HTTP protocol
- **Unidirectional**: Matches MCP request/response pattern

### Why JSON-RPC 2.0?

- **Standardization**: Well-defined protocol specification
- **Type Safety**: Clear parameter and response formats
- **Error Handling**: Structured error reporting
- **Tooling**: Extensive client library support

## Version

Current version: `0.1.0`

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.