# MCP Testing Guide

Complete guide for testing MCP (Model Context Protocol) functionality over Server-Sent Events (SSE) transport.

## Overview

This project implements a Rust-based MCP Task Management Server with the following architecture:
- **HTTP POST** for client-to-server requests (JSON-RPC 2.0)
- **Server-Sent Events (SSE)** for server-to-client responses and notifications
- **Session management** with Session-ID and Event-ID for connection resumption

## Testing Strategy

### 1. Automated Integration Tests

**Location**: `mcp-server/tests/mcp_integration_tests.rs`

**Run Tests**:
```bash
# Run all MCP integration tests
cargo test -p mcp-server mcp_integration

# Run specific test with output
cargo test -p mcp-server test_mcp_task_lifecycle -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test -p mcp-server mcp_integration
```

**Test Coverage**:
- ✅ Complete task lifecycle (create, update, state changes, archive)
- ✅ Error handling (invalid methods, parameters, not found)
- ✅ Health check functionality
- ✅ JSON-RPC 2.0 compliance
- ✅ SSE event processing
- ✅ Session management

### 2. Manual Testing with MCP Inspector

**Installation**:
```bash
# Install Node.js if not already installed
# Then run the MCP Inspector
npx @modelcontextprotocol/inspector
```

**Usage**:
1. Start your MCP server: `cargo run -p mcp-server`
2. Run the inspector: `npx @modelcontextprotocol/inspector http://127.0.0.1:8080`
3. Configure endpoints:
   - **Request Endpoint**: `/mcp/request`
   - **SSE Endpoint**: `/mcp/v1`
4. Send test requests and observe SSE responses in real-time

### 3. Manual Testing with curl + SSE Tools

**Send MCP Request**:
```bash
# Create a task
curl -X POST http://127.0.0.1:8080/mcp/request \
  -H "Content-Type: application/json" \
  -H "Origin: http://127.0.0.1:8080" \
  -d '{
    "jsonrpc": "2.0",
    "method": "create_task",
    "params": {
      "code": "CURL-001",
      "name": "Test Task via curl",
      "description": "Testing MCP with curl",
      "owner_agent_name": "curl-user"
    },
    "id": 1
  }'
```

**Listen to SSE Stream**:
```bash
# Listen to SSE responses (use in separate terminal)
curl -N -H "Accept: text/event-stream" \
     -H "Origin: http://127.0.0.1:8080" \
     http://127.0.0.1:8080/mcp/v1
```

## MCP Protocol Compliance

### Transport: Streamable HTTP

Based on MCP specification 2024-11-05+, the protocol uses:
- **HTTP POST** for requests to `/mcp/request`
- **Server-Sent Events** for responses via `/mcp/v1`
- **Session management** via `Session-ID` header
- **Event replay** via `Last-Event-ID` header

### JSON-RPC 2.0 Structure

**Request Format**:
```json
{
  "jsonrpc": "2.0",
  "method": "create_task",
  "params": { ... },
  "id": 1
}
```

**Response Format**:
```json
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}
```

**Error Format**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Method not found"
  },
  "id": 1
}
```

### Available MCP Methods

#### Task Management
- `create_task` - Create a new task
- `update_task` - Update existing task
- `set_task_state` - Change task state
- `get_task_by_id` - Retrieve task by ID
- `get_task_by_code` - Retrieve task by code
- `list_tasks` - List tasks with filters
- `assign_task` - Assign task to agent
- `archive_task` - Archive completed task

#### System
- `health_check` - Check server health

### Example Test Scenarios

#### Happy Path: Complete Task Lifecycle
```javascript
// 1. Create task
POST /mcp/request
{
  "jsonrpc": "2.0",
  "method": "create_task",
  "params": {
    "code": "TASK-001",
    "name": "Sample Task",
    "description": "Test task creation",
    "owner_agent_name": "test-agent"
  },
  "id": 1
}

// 2. Update task
{
  "jsonrpc": "2.0", 
  "method": "update_task",
  "params": {
    "id": 1,
    "name": "Updated Sample Task"
  },
  "id": 2
}

// 3. Change state
{
  "jsonrpc": "2.0",
  "method": "set_task_state", 
  "params": {
    "id": 1,
    "state": "InProgress"
  },
  "id": 3
}

// 4. List tasks
{
  "jsonrpc": "2.0",
  "method": "list_tasks",
  "params": {
    "owner_agent_name": "test-agent"
  },
  "id": 4
}
```

#### Error Handling Tests
```javascript
// Invalid method
{
  "jsonrpc": "2.0",
  "method": "invalid_method",
  "id": 1
}
// Expected: {"error": {"code": -32601, "message": "Method not found"}}

// Invalid parameters
{
  "jsonrpc": "2.0",
  "method": "create_task",
  "params": {
    "invalid_field": "value"
  },
  "id": 2
}
// Expected: {"error": {"code": -32602, "message": "Invalid params"}}
```

## Performance Testing

### Load Testing with Custom Client

```rust
// Example: Concurrent task creation
#[tokio::test]
async fn test_concurrent_task_creation() {
    let server_url = start_test_server().await;
    let num_clients = 10;
    let tasks_per_client = 100;
    
    let handles: Vec<_> = (0..num_clients).map(|client_id| {
        let server_url = server_url.clone();
        tokio::spawn(async move {
            let mut client = McpTestClient::new(&server_url).unwrap();
            for i in 0..tasks_per_client {
                let params = json!({
                    "code": format!("CLIENT-{}-TASK-{}", client_id, i),
                    "name": format!("Task {} from client {}", i, client_id),
                    "description": "Load test task",
                    "owner_agent_name": format!("client-{}", client_id)
                });
                client.send_mcp_request("create_task", Some(params), i as u64).await?;
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }).collect();
    
    // Wait for all clients to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
}
```

## Security Testing

### Origin Header Validation
```bash
# Valid origin (should work)
curl -X POST http://127.0.0.1:8080/mcp/request \
  -H "Origin: http://127.0.0.1:8080" \
  -d '{"jsonrpc":"2.0","method":"health_check","id":1}'

# Invalid origin (should be rejected)
curl -X POST http://127.0.0.1:8080/mcp/request \
  -H "Origin: http://malicious-site.com" \
  -d '{"jsonrpc":"2.0","method":"health_check","id":1}'
```

### Input Validation
```bash
# SQL injection attempt (should be safely handled)
curl -X POST http://127.0.0.1:8080/mcp/request \
  -H "Origin: http://127.0.0.1:8080" \
  -d '{
    "jsonrpc": "2.0",
    "method": "get_task_by_code",
    "params": {
      "code": "'; DROP TABLE tasks; --"
    },
    "id": 1
  }'
```

## Debugging Tips

### Enable Debug Logging
```bash
RUST_LOG=debug cargo run -p mcp-server
```

### SSE Connection Debugging
```bash
# Monitor SSE events with timestamps
curl -N -H "Accept: text/event-stream" \
     -H "Origin: http://127.0.0.1:8080" \
     http://127.0.0.1:8080/mcp/v1 | \
     while IFS= read -r line; do
       echo "[$(date)] $line"
     done
```

### JSON-RPC Request/Response Validation
Use online JSON-RPC validators or:
```javascript
// Validate JSON-RPC structure
function validateJsonRpc(obj) {
  if (obj.jsonrpc !== "2.0") return false;
  if (!obj.method && !obj.result && !obj.error) return false;
  if (obj.method && (!obj.params || !obj.id)) return false;
  return true;
}
```

## Troubleshooting

### Common Issues

1. **SSE Connection Refused**
   - Check server is running: `netstat -an | grep 8080`
   - Verify CORS/Origin headers
   - Check firewall settings

2. **JSON-RPC Parse Errors**
   - Validate JSON syntax
   - Ensure `Content-Type: application/json`
   - Check for missing required fields

3. **Session Management Issues**
   - Verify `Session-ID` header is included
   - Check `Last-Event-ID` for reconnection
   - Monitor server logs for session lifecycle

4. **Test Failures**
   - Use `--nocapture` flag to see println! output
   - Enable debug logging with `RUST_LOG=debug`
   - Check if test database is properly isolated

### Performance Issues

1. **Slow Response Times**
   - Monitor database query performance
   - Check SSE connection pool limits  
   - Profile async task scheduling

2. **Memory Leaks**
   - Monitor SSE connection cleanup
   - Check task spawning vs completion
   - Use memory profilers like `heaptrack`

## Best Practices

1. **Test Isolation**: Use in-memory databases for tests
2. **Concurrent Testing**: Test multiple client scenarios
3. **Error Coverage**: Test all error code paths
4. **Session Testing**: Test reconnection and state recovery
5. **Performance Monitoring**: Include latency and throughput metrics
6. **Security Testing**: Validate all input sanitization
7. **Protocol Compliance**: Ensure strict JSON-RPC 2.0 adherence

## Continuous Integration

Add to your CI pipeline:
```yaml
- name: Run MCP Integration Tests
  run: |
    cargo test -p mcp-server mcp_integration -- --test-threads=1
    
- name: Test MCP Protocol Compliance
  run: |
    # Start server in background
    cargo run -p mcp-server &
    SERVER_PID=$!
    sleep 5
    
    # Run protocol compliance tests
    npx @modelcontextprotocol/test-suite http://127.0.0.1:8080
    
    # Cleanup
    kill $SERVER_PID
```

This comprehensive testing approach ensures your MCP server is robust, compliant, and production-ready!