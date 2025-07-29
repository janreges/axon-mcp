# STDIO MCP Protocol Testing Guide

Kompletní testovací sada pro STDIO transport MCP Task Management Serveru.

## Přehled

MCP server podporuje dual transport:
- **HTTP/SSE transport**: `./mcp-server` (default)
- **STDIO transport**: `./mcp-server --transport stdio`

STDIO transport používá JSON-RPC 2.0 protokol přes stdin/stdout podle MCP specifikace.

## Testovací nástroje

### 1. Automatické testy (test_mcp.sh)

```bash
# Spustit všechny STDIO testy
./test_mcp.sh stdio

# Spustit kompletní test suite (včetně STDIO)
./test_mcp.sh all

# Spustit pouze HTTP testy (bez STDIO)
./test_mcp.sh integration
```

### 2. Manuální testy (test_stdio_manual.sh)

```bash
# Interaktivní menu
./test_stdio_manual.sh

# Jednotlivé test typy
./test_stdio_manual.sh handshake   # Pouze handshake
./test_stdio_manual.sh tools       # Všechny nástroje
./test_stdio_manual.sh workflow    # Kompletní workflow
./test_stdio_manual.sh interactive # Interaktivní session
```

## Test Cases

### 1. MCP Protocol Handshake

Testuje základní MCP protokol flow:

```json
// 1. Initialize request
{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "test-client", "version": "1.0.0"}}, "id": 1}

// 2. Server response s capabilities
{"jsonrpc": "2.0", "result": {"protocolVersion": "2024-11-05", "capabilities": {...}, "serverInfo": {...}}, "id": 1}

// 3. Initialized notification
{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}
```

**Testuje:**
- ✅ Správná JSON-RPC 2.0 struktura
- ✅ Protocol version compliance (2024-11-05)
- ✅ Server capabilities listing
- ✅ State management (WaitingForInitialize → WaitingForInitialized → Ready)

### 2. MCP Tools Testing

Testuje všech 9 MCP nástrojů:

| Tool | Test Case | Expected Result |
|------|-----------|----------------|
| `health_check` | `{}` | `{"status": "healthy"}` |
| `create_task` | Required params | Task object with ID |
| `update_task` | ID + optional params | Updated task object |
| `set_task_state` | ID + state enum | Task with new state |
| `get_task_by_id` | Numeric ID | Task object or null |
| `get_task_by_code` | String code | Task object or null |
| `list_tasks` | Optional filters | Array of tasks |
| `assign_task` | ID + new owner | Task with new owner |
| `archive_task` | Task ID | Archived task |

**Formát tool calls:**
```json
{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "TOOL_NAME", "arguments": {...}}, "id": N}
```

### 3. Error Handling

Testuje různé error scenarios:

| Error Type | Test Case | Expected JSON-RPC Error |
|------------|-----------|------------------------|
| Parse Error | `invalid json` | `-32700` |
| Invalid Request | Missing jsonrpc | `-32600` |
| Method Not Found | `unknown_method` | `-32601` |
| Invalid Params | Missing required fields | `-32602` |
| Internal Error | Server-side failures | `-32603` |
| Protocol State | Tool call before initialize | Custom error |

**Error Response Format:**
```json
{"jsonrpc": "2.0", "error": {"code": -32700, "message": "Parse error"}, "id": null}
```

### 4. Concurrent Access

Testuje současný přístup více STDIO sessions:
- ✅ 3 paralelní MCP sessions
- ✅ Každá session má vlastní initialize/initialized flow
- ✅ Současné vytváření tasků bez konfliktů
- ✅ Database consistency při concurrent přístupu

### 5. State Transition Testing

Testuje task lifecycle:

```
Created → InProgress → Review → Done → Archived
    ↓         ↓         ↓      ↓
  Blocked   Blocked   Blocked  ↗
```

**Test workflow:**
1. Create task (state: Created)
2. Set state InProgress
3. Update task details
4. Assign to different agent
5. Set state Done
6. Archive task (state: Archived)

## Spuštění testů

### Základní použití

```bash
# Build server
cargo build -p mcp-server --release

# Automatické testy
./test_mcp.sh stdio

# Manuální testování
./test_stdio_manual.sh
```

### Příklad ruční session

```bash
# Spusť server
./target/release/mcp-server --transport stdio

# V jiném terminálu nebo pipe commands:
echo '{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "manual", "version": "1.0.0"}}, "id": 1}' | ./target/release/mcp-server --transport stdio
```

### Debug a troubleshooting

**Server logging:**
```bash
# Verbose logging
RUST_LOG=debug ./target/release/mcp-server --transport stdio

# JSON formatting
echo 'REQUEST' | ./target/release/mcp-server --transport stdio | jq .
```

**Common issues:**
- **No response**: Check JSON-RPC format, ensure proper line endings
- **Parse errors**: Validate JSON syntax, check quotes
- **State errors**: Ensure proper initialize → initialized flow
- **Timeout**: Increase timeout in test scripts

## Výsledky testů

### Test output interpreting

**Úspěšný test:**
```
✅ MCP Protocol
✅ All Tools  
✅ Error Handling
✅ Concurrent Access

STDIO Test Results Summary:
  ✅ MCP Protocol
  ✅ All Tools
  ✅ Error Handling  
  ✅ Concurrent Access

✅ All STDIO tests passed successfully!
```

**Test s chybami:**
```
❌ MCP Protocol
⚠️  Some tests may have issues

STDIO Test Results Summary:
  ❌ MCP Protocol
  ✅ All Tools
  ✅ Error Handling
  ✅ Concurrent Access

❌ 1 STDIO test(s) failed
```

### Performance expectations

- **Handshake**: < 1s
- **Tool calls**: < 500ms per call
- **Concurrent sessions**: 3+ simultaneous
- **Error handling**: Immediate error responses
- **Large payloads**: Support for complex task objects

## CI/CD Integration

Testy jsou integrovány do `./test_mcp.sh all`:

```bash
# Complete test suite order:
1. Integration tests (Rust unit tests)
2. HTTP server tests (curl, SSE)
3. Performance tests (HTTP load)
4. STDIO MCP tests (JSON-RPC over stdin/stdout)
5. Inspector setup validation
```

**GitHub Actions usage:**
```yaml
- name: Run MCP tests
  run: |
    cargo build --release
    ./test_mcp.sh all
```

## Rozšíření testů

### Přidání nového testu

1. **Automatický test** (test_mcp.sh):
```bash
test_new_feature() {
    print_info "Testing new feature..."
    # Implementation
}

# Add to run_stdio_tests()
if test_new_feature; then
    test_results+=("✅ New Feature")
else
    test_results+=("❌ New Feature")
fi
```

2. **Manuální test** (test_stdio_manual.sh):
```bash
test_tool "new_tool" '{"param": "value"}' "Description"
```

### Custom test scenarios

```bash
# Vlastní test script
#!/bin/bash
(
    echo 'INITIALIZE_MESSAGE'
    echo 'INITIALIZED_NOTIFICATION'
    echo 'CUSTOM_TOOL_CALLS'
    sleep 2
) | timeout 10 ./target/release/mcp-server --transport stdio
```

Testy pokrývají kompletní MCP protokol specifikaci a zajistí robustní STDIO transport implementation.