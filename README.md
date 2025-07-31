# 🧠 Axon
## The MCP Hub for Task & Message Coordination Between AI Agents

[![Build Status](https://img.shields.io/github/actions/workflow/status/janreges/axon-mcp/rust.yml?branch=main&style=for-the-badge)](https://github.com/janreges/axon-mcp/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/axon-mcp?style=for-the-badge&logo=rust)]()

Axon is a production-grade Model Context Protocol (MCP) server written in Rust. It acts as the single hub where your AI agents **create, claim, and track tasks while exchanging structured handoffs, questions, and blockers** in real-time. With dual transports (HTTP/SSE + JSON-RPC/STDIO), <100ms latency, and 15 first-class MCP functions, Axon lets you orchestrate small agent squads or large autonomous swarms—all backed by an ACID-compliant SQLite core.

Think of Axon as the neural relay between specialized agents—just add well-crafted prompts and watch them collaborate like a team.

---

## ✨ Why Axon?

| **Challenge** | **Axon's Solution** |
|-------------|-------------------|
| "My agents talk past each other" | 🎯 **Targeted messaging** within task contexts |
| "I lose context between agent handoffs" | 💾 **Persistent task state** with full audit trail |
| "Scaling from 1→N agents is chaotic" | 🔄 **Task discovery & claiming** with capability matching |
| "Hard to debug multi-agent workflows" | 📊 **Built-in timeline** with trace IDs and message threading |
| "Complex setup and dependencies" | ⚡ **Zero-config SQLite** backend, single binary deployment |

---

## 🚀 Quick Start

### Prerequisites
- Rust 1.75+ 
- SQLite 3.x (usually pre-installed)

### Installation & Run

```bash
# Clone and build
git clone https://github.com/janreges/axon-mcp.git
cd axon-mcp
cargo build --release

# Run HTTP/SSE mode (default)
./target/release/mcp-server
# Database auto-created at ~/db.sqlite

# Or run STDIO mode for direct MCP integration
./target/release/mcp-server --transport stdio
```

### Configuration

```bash
# Custom database path
export DATABASE_URL="sqlite:///path/to/axon.db"
./target/release/mcp-server

# Or via command line
./target/release/mcp-server --database-url sqlite:///custom/path/tasks.db
```

**🎉 That's it!** Axon is now running and ready to coordinate your AI agents.

---

## 🤖 Core Use Cases

### 1. 🏗️ **Hierarchical Agent Teams**
*A Manager agent orchestrates specialized Worker agents*

**Scenario**: "Analyze competitor's product launch and write a summary report"

1. **Manager Agent**: Creates main task + sub-tasks for research, analysis, writing
2. **Research Agent**: Claims research task, gathers data, sends `handoff` message with findings
3. **Analysis Agent**: Receives handoff, analyzes data, updates task to `Done`
4. **Writer Agent**: Gets notification, drafts report based on all previous work

```json
// Manager creates task
{"method": "create_task", "params": {"code": "COMPETITOR-001", "name": "Product Launch Analysis"}}

// Research agent sends findings
{"method": "create_task_message", "params": {
  "task_code": "COMPETITOR-001",
  "author_agent_name": "research-agent", 
  "target_agent_name": "analysis-agent",
  "message_type": "handoff",
  "content": "Found 5 key features: {...}"
}}
```

### 2. 🔄 **Sequential Processing Pipeline**
*Agents process work in stages like a CI/CD pipeline*

**Scenario**: Code generation → Review → Testing → Deployment

`code-generator` → `code-reviewer` → `testing-agent` → `deployment-agent`

Each agent:
1. Claims next available task with `claim_task`
2. Processes the work 
3. Sends results via `create_task_message`
4. Updates task state for next agent in pipeline

### 3. 🧠 **Parallel Brainstorming**
*Multiple agents work on the same problem simultaneously*

**Scenario**: "Research 3 different approaches to solve X"

- Manager spawns 3 identical research tasks
- Research agents claim tasks simultaneously  
- Each contributes findings via messages
- Synthesis agent combines all results

---

## 🔌 Agent Prompt Engineering (Critical!)

**Axon provides the coordination infrastructure, but your agents need proper instructions to use it effectively.**

### Example System Prompt for a Research Agent:

```text
You are 'research-agent', specialized in gathering and analyzing information.

AXON INTEGRATION:
You can coordinate with other agents through these MCP functions:

- list_tasks({"owner": "research-agent", "state": "Created"}) - Find your assigned tasks
- get_task_by_id({"id": N}) - Get task details  
- set_task_state({"id": N, "state": "InProgress"}) - Start working
- create_task_message({
    "task_code": "TASK-001", 
    "author_agent_name": "research-agent",
    "target_agent_name": "analysis-agent", 
    "message_type": "handoff",
    "content": "Your findings here..."
  }) - Share results with other agents
- set_task_state({"id": N, "state": "Done"}) - Mark complete

WORKFLOW:
1. Check for new tasks assigned to you
2. Set task to "InProgress" 
3. Do your research work
4. Send findings to the next agent via message
5. Mark task as "Done"

Always include context and clear handoffs in your messages.
```

**💡 Pro Tips:**
- Define clear **message types** (`handoff`, `question`, `blocker`, `comment`)
- Use **targeted messages** to avoid noise
- Include **task codes** for traceability  
- Handle **error states** gracefully

---

## 📋 Complete MCP Function Reference

Axon implements **15 comprehensive MCP functions** organized in three categories:

<details>
<summary><strong>📝 Core Task Management (8 Functions)</strong></summary>

- **`create_task`** - Create new tasks with validation
- **`update_task`** - Modify task metadata and descriptions  
- **`assign_task`** - Transfer ownership between agents
- **`get_task_by_id`** - Retrieve task by numeric ID
- **`get_task_by_code`** - Retrieve task by human-readable code (e.g., `TASK-123`)
- **`list_tasks`** - Query tasks with filters (owner, state, date range)
- **`set_task_state`** - Change task lifecycle state with validation
- **`archive_task`** - Move completed tasks to archive

</details>

<details>
<summary><strong>🤝 Multi-Agent Coordination (5 Functions)</strong></summary>

- **`discover_work`** - Find available tasks based on agent capabilities
- **`claim_task`** - Atomically claim unassigned tasks for execution
- **`release_task`** - Release claimed tasks back to the pool
- **`start_work_session`** - Begin time tracking for task work
- **`end_work_session`** - Complete work session with productivity metrics

</details>

<details>
<summary><strong>💬 Inter-Agent Messaging (2 Functions)</strong></summary>

- **`create_task_message`** - Send targeted messages within task contexts
  - **Types**: `handoff`, `question`, `comment`, `solution`, `blocker`, custom
  - **Threading**: Reply chains with `reply_to_message_id`
- **`get_task_messages`** - Retrieve messages with advanced filtering
  - Filter by sender, recipient, message type, threading

</details>

📖 **Full API documentation with JSON examples**: [`docs/API.md`](docs/API.md)

---

## 🏗️ Architecture

**Multi-crate Rust workspace** designed for performance and maintainability:

```
axon-mcp/
├── core/           # 🧩 Domain models and business logic  
├── database/       # 🗄️ SQLite repository implementation
├── mcp-protocol/   # 🌐 MCP server with HTTP/SSE transport
├── mcp-server/     # 🚀 Main binary and configuration
└── mocks/          # 🧪 Test utilities and fixtures
```

**Transport Modes:**
- **HTTP/SSE**: Web dashboard + Server-Sent Events for MCP
- **STDIO**: JSON-RPC over stdin/stdout for direct integration

**Task State Machine:**
```
Created → InProgress → Review → Done → Archived
    ↓         ↓          ↓       ↓
  Blocked ←---+----------+-------+
```

---

## 📊 Performance & Scale

- **Response Time**: <100ms for single operations
- **Throughput**: 1000+ operations per second  
- **Concurrent Agents**: 100+ simultaneous connections
- **Database Capacity**: 1M+ tasks and messages without degradation
- **Message Filtering**: Optimized indexes for fast targeted retrieval

---

## 🎯 Real-World Applications

**🔬 Complex Analysis Projects**
- Research → Analysis → Report generation workflows
- Multi-perspective evaluation (technical, business, legal teams)

**💻 Software Development**  
- Code generation → Review → Testing → Deployment pipelines
- Architecture planning with specialized expert agents

**📈 Business Intelligence**
- Data collection → Processing → Visualization → Insights
- Parallel analysis by domain experts

**🧪 Research & Development**
- Hypothesis generation → Testing → Validation workflows
- Literature review + experimental design coordination

---

## 🛠️ Development

### Building from Source

```bash
# Build all crates
cargo build --workspace

# Run comprehensive tests  
cargo test --workspace

# Development with logging
RUST_LOG=debug cargo run --bin mcp-server
```

### Testing

```bash
# Unit tests per crate
cargo test --lib

# Integration tests
cargo test --test integration  

# Coverage report
cargo tarpaulin --out html
```

---

## 🤝 Contributing

We welcome contributions! Please see [`CONTRIBUTING.md`](CONTRIBUTING.md) for:
- Development guidelines
- Testing procedures  
- Submission requirements

**Quick Steps:**
1. Fork and create feature branch
2. `cargo fmt && cargo clippy` (we auto-lint)
3. Add tests for new functionality
4. Open PR with clear description

---

## 📄 License

MIT © 2025 Jan Reges & Contributors

See the [`LICENSE`](LICENSE) file for details.

---

## 🔗 Links

- **Repository**: https://github.com/janreges/axon-mcp
- **Documentation**: [`docs/`](docs/) 
- **Issues & Discussion**: [GitHub Issues](https://github.com/janreges/axon-mcp/issues)
- **Model Context Protocol**: [MCP Specification](https://github.com/modelcontextprotocol/mcp-spec)

---

*🧠 Axon: Where AI agents connect, coordinate, and collaborate.*