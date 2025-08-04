# Axon MCP – Task Management Server for AI Agents

Welcome to the **Axon MCP** server – a production-ready Model Context Protocol (MCP) server designed for coordinating multiple AI agents working in parallel.

**Why HTTP-only?** Traditional STDIO-based MCP servers can't handle multiple concurrent requests needed for multi-agent coordination, task discovery, and long-polling workflows. Axon uses HTTP transport to enable simultaneous agent connections and real-time task coordination.

---

## 1. Overview

|                 | Traditional MCPs        | Axon MCP             |
|-----------------|-------------------------|----------------------|
| Transport       | STDIO (single request)  | **HTTP (concurrent)** |
| Use Case        | Single agent interaction| **Multi-agent coordination** |
| State storage   | Flat files              | **`.axon/axon.<PROJECT>.sqlite`** |
| Integration     | Custom configuration    | **`claude mcp add`** (built-in) |

Key design goals:

* **Multi-agent support** – HTTP transport enables concurrent requests from multiple agents
* **Project-scoped data** – each project gets its own lightweight SQLite DB under `.axon/`
* **Complete MCP implementation** – all 22 MCP functions available via JSON-RPC 2.0
* **Production ready** – structured logging, health checks, graceful shutdown
* **Zero configuration** – automatically creates `.axon/` and `.claude/` directories

---

## 2. Quick Install (curl)

```bash
# 1) Download & install the latest binary (macOS / Linux x86_64)
curl -s https://raw.githubusercontent.com/janreges/axon-mcp/main/install.sh | bash

# 2) The script prints the final path, e.g.
# Installed axon-mcp to /usr/local/bin/axon-mcp
```

Keep the printed path handy – you'll need it in the next step.

---

## 3. Usage Workflow

### 3.1 Start the MCP server

```bash
axon-mcp --start \
  --port=8888 \
  --project=my-project \
  --project-root=/path/to/project
```

What happens:

* `.axon/` and `.claude/` folders are created if missing.
* A SQLite DB named `.axon/axon.my-project.sqlite` is initialised.
* Server listens on `http://localhost:8888`.

### 3.2 Hook Claude into the running server

```bash
cd /path/to/project
claude mcp add --url http://localhost:8888
```

Claude now forwards all MCP calls over HTTP; no further setup required.

---

## 4. Feature Highlights

* **Multi-agent coordination** – HTTP transport supports concurrent requests from multiple agents
* **Task discovery & claiming** – agents can find and atomically claim work based on capabilities
* **Inter-agent messaging** – targeted communication within task contexts with threading support
* **Per-project isolation** – each project gets its own database, no cross-project data leakage
* **Complete MCP spec** – all 22 MCP functions available via JSON-RPC over HTTP
* **Production logging** – structured request logs with timing and parameter truncation

### 4.1 Structured Request Logging Example

When you run commands, you'll see clean, single-line logs like:

```
2025-08-04 13:16:30 [health_check] [0 ms]
2025-08-04 13:16:36 [create_task] [0 ms] code="TASK-001", description="Long description text...", name="Task name..."
2025-08-04 13:17:02 [create_task_message] [0 ms] author_agent_name="[REDACTED]", content="Message content...", message_type="handoff"
```

Features:
- **Timestamp** in YYYY-MM-DD HH:MM:SS format
- **Function name** in brackets
- **Execution time** in milliseconds
- **Parameters** with strings truncated to 30 characters + "..."
- **Sensitive data redaction** for security (passwords, tokens, keys)
- **Array formatting** as `[N items]` for clean output

---

## 5. Complete MCP Function Reference

Axon implements **22 comprehensive MCP functions** organized in four categories:

### 📝 Core Task Management (9 Functions)
- **`create_task`** - Create new tasks with validation
- **`update_task`** - Modify task metadata and descriptions  
- **`assign_task`** - Transfer ownership between agents
- **`get_task_by_id`** - Retrieve task by numeric ID
- **`get_task_by_code`** - Retrieve task by human-readable code (e.g., `TASK-123`)
- **`list_tasks`** - Query tasks with filters (owner, state, date range)
- **`set_task_state`** - Change task lifecycle state with validation
- **`archive_task`** - Move completed tasks to archive
- **`health_check`** - Check server health and status

### 🤝 Multi-Agent Coordination (5 Functions)
- **`discover_work`** - Find available tasks based on agent capabilities
- **`claim_task`** - Atomically claim unassigned tasks for execution
- **`release_task`** - Release claimed tasks back to the pool
- **`start_work_session`** - Begin time tracking for task work
- **`end_work_session`** - Complete work session with productivity metrics

### 💬 Inter-Agent Messaging (2 Functions)
- **`create_task_message`** - Send targeted messages within task contexts
  - **Types**: `handoff`, `question`, `comment`, `solution`, `blocker`, custom
  - **Threading**: Reply chains with `reply_to_message_id`
- **`get_task_messages`** - Retrieve messages with advanced filtering
  - Filter by sender, recipient, message type, threading

### 🚀 Workspace Setup Automation (6 Functions)
- **`get_setup_instructions`** - Generate AI workspace setup instructions
- **`get_agentic_workflow_description`** - Generate agent workflow recommendations
- **`register_agent`** - Register AI agent with capabilities and contact info
- **`get_instructions_for_main_ai_file`** - Get template for main coordination file
- **`create_main_ai_file`** - Generate main AI coordination file (CLAUDE.md, etc.)
- **`get_workspace_manifest`** - Generate complete workspace manifest

---

## 6. Architecture

**Multi-crate Rust workspace** designed for performance and maintainability:

```
axon-mcp/
├── core/           # 🧩 Domain models and business logic  
├── database/       # 🗄️ SQLite repository implementation
├── mcp-protocol/   # 🌐 MCP server with HTTP transport
├── mcp-server/     # 🚀 Main binary and configuration
└── mocks/          # 🧪 Test utilities and fixtures
```

**Key Features:**
- **Concurrent request support**: HTTP enables multiple agents to work simultaneously
- **Task coordination**: Atomic claiming, work sessions, and inter-agent messaging
- **Project-scoped databases**: Each project gets its own SQLite file
- **Production ready**: Structured logging, health checks, graceful shutdown

---

## 7. Troubleshooting

| Symptom | Possible Fix |
|---------|--------------|
| `Port 8888 already in use` | Pick another port: `--port=9090` |
| `axon-mcp: command not found` | Confirm the install path shown by the curl script is on `$PATH`, or invoke it with the absolute path. |
| `claude mcp add` fails to connect | • Verify the MCP server URL and port.<br>• Ensure the server is running in the same network namespace.<br>• Check for proxy/firewall rules blocking localhost. |
| Database locked errors | Rare on local FS; if seen, ensure only one server instance points at the same `.axon/*.sqlite`. |
| Need verbose output | Start the server with `--log-level=debug`. |

---

## 8. Example Usage with curl

Once your server is running, you can test it with curl:

```bash
# Health check
curl -X POST http://127.0.0.1:8888/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-03-26" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "health_check",
    "params": {}
  }'

# Create a task
curl -X POST http://127.0.0.1:8888/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-03-26" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "create_task",
    "params": {
      "code": "DEMO-001",
      "name": "Example Task",
      "description": "This is a demo task created via HTTP"
    }
  }'

# List tasks
curl -X POST http://127.0.0.1:8888/mcp \
  -H "Content-Type: application/json" \
  -H "MCP-Protocol-Version: 2025-03-26" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "list_tasks",
    "params": {
      "limit": 10
    }
  }'
```

---

## 9. Development

### Building from Source

```bash
# Clone and build
git clone https://github.com/janreges/axon-mcp.git
cd axon-mcp
cargo build --workspace

# Run tests
cargo test --workspace

# Start development server
cargo run --bin axon-mcp -- --start --port=8888 --project=dev --project-root=/tmp/dev-project
```

### Useful Make Commands

```bash
make help                    # Show all available commands
make build                   # Build all crates
make test                    # Run all tests
make check-status           # Show project status
```

---

## 10. Contributing & Feedback

We love PRs, issues and suggestions.  
Open a ticket or contribute at [GitHub Issues](https://github.com/janreges/axon-mcp/issues).

---

## 11. License

MIT © 2025 Jan Reges & Contributors. See [LICENSE](LICENSE) for full text.

---

*🧠 Axon MCP: Task coordination and management for AI agents.*