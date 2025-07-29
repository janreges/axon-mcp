# Professional Rust MCP Development Project

## 🚨 CRITICAL INSTRUCTION FOR CONTROL AGENT 🚨

**YOU MUST IMMEDIATELY ACTIVATE ALL 8 AGENTS IN PARALLEL**

**DO NOT DO ANY WORK YOURSELF - YOU ARE ONLY A COORDINATOR**

1. **ACTIVATE ALL AGENTS SIMULTANEOUSLY**: Launch all 8 agents at the same time using the Task tool
2. **NO SOLO WORK**: You are FORBIDDEN from writing code, documentation, or implementing anything yourself
3. **NO SIMULATION**: You CANNOT pretend to be any agent or call log.sh as if you were an agent
4. **ONLY COORDINATE**: Your role is purely coordination - activate agents and let them work directly with each other

**MANDATORY FIRST ACTION**: Use the Task tool to launch ALL 8 agents simultaneously with these exact instructions:

```
You are working on a professional Rust MCP development project. Start immediately and coordinate directly with other agents using ./log.sh. Work in aggressive parallel mode - do not wait for others to finish before starting your work. Communicate continuously with other team members about your progress and needs.
```

## Project Overview

Develop a production-ready Model Context Protocol (MCP) server in Rust for comprehensive task management. This server provides essential task tracking, assignment, and lifecycle management capabilities through a clean, multi-crate architecture that enables parallel development by specialized teams.

## Technical Specifications

### Core Task Data Model
```rust
struct Task {
    id: i32,                    // Auto-increment primary key
    code: String,               // Human-readable identifier (e.g., "ARCH-01", "DB-15")
    name: String,               // Brief task title
    description: String,        // Detailed task requirements
    owner_agent_name: String,   // Assigned agent identifier
    state: TaskState,           // Current lifecycle state
    inserted_at: DateTime<Utc>, // Creation timestamp
    done_at: Option<DateTime<Utc>>, // Completion timestamp
}

enum TaskState {
    Created,
    InProgress,
    Blocked,
    Review,
    Done,
    Archived,
}
```

### Required MCP Functions
- **create_task**: Add new task with validation
- **update_task**: Modify task details and metadata
- **set_task_state**: Change task lifecycle state
- **get_task_by_id**: Retrieve task by numeric ID
- **get_task_by_code**: Retrieve task by human-readable code
- **list_tasks**: Query tasks with filtering (owner, state, date range)
- **assign_task**: Transfer task ownership between agents
- **archive_task**: Move task to archived state with audit trail

### Technology Stack
- **Framework**: Rust with MCP SDK (https://github.com/modelcontextprotocol/rust-sdk)
- **Database**: SQLite with automatic path handling (~/db.sqlite default)
- **Transport**: Server-Sent Events (SSE) for MCP communication
- **Serialization**: JSON for MCP protocol compliance
- **Testing**: Comprehensive unit, integration, and contract tests
- **Documentation**: Full API documentation with examples

## Multi-Crate Architecture

The project is structured as a Rust workspace with 5 independent crates, each owned by specialized agent teams:

### Crate Ownership

**1. `core` crate** - Owners: rust-architect + backend-developer
   - Domain models, business logic, trait interfaces
   - Task struct, TaskState enum, error types
   - Repository and protocol handler traits

**2. `database` crate** - Owner: database-designer
   - SQLite implementation of TaskRepository trait
   - Database migrations and schema management
   - Connection pooling and error mapping

**3. `mcp-protocol` crate** - Owner: mcp-integrator
   - MCP server implementation with SSE transport
   - Protocol handler implementation
   - JSON-RPC message handling

**4. `mcp-server` crate** - Owner: git-coordinator
   - Main binary assembling all components
   - Configuration management
   - Dependency injection and startup logic

**5. `mocks` crate** - Owner: qa-tester
   - Mock implementations for testing
   - Test fixtures and generators
   - Contract test helpers

### Supporting Team Members

**documentation-specialist** - Creates PRD.md, ARCHITECTURE.md, API documentation
**devops-engineer** - CI/CD pipelines, Docker support, deployment automation

## Development Methodology

### MANDATORY Aggressive Parallel Development

**CONTROL AGENT REQUIREMENTS**:
- **IMMEDIATELY launch ALL 5 crate owners using Task tool** - no delays, no sequential activation
- **DO NOT DO ANY WORK YOURSELF** - you only coordinate, never implement
- **NO AGENT SIMULATION** - do not pretend to be any agent or use their log.sh
- **LET AGENTS COMMUNICATE DIRECTLY** - do not mediate their conversations

**CRATE OWNER REQUIREMENTS**:
- **START IMMEDIATELY**: Begin work on your crate the moment you are activated
- **FOLLOW YOUR TASKLIST**: Each crate has a detailed TASKLIST.[crate-name].md file
- **COMMUNICATE DIRECTLY**: Use ./log.sh to coordinate with other crate owners
- **WORK IN PARALLEL**: Do not wait for others - develop against trait interfaces
- **MARK PROGRESS**: Update your TASKLIST as you complete items

### Communication Requirements
**ALL team members MUST use `./log.sh "message"` for coordination logging**

Examples:
- `./log.sh "CORE-ARCHITECT → DATABASE-DESIGNER: TaskRepository trait defined, ready for implementation"`
- `./log.sh "DATABASE-DESIGNER → QA-TESTER: Need mock repository for testing"`
- `./log.sh "MCP-INTEGRATOR → ALL: SSE endpoint ready at /mcp/v1"`
- `./log.sh "GIT-COORDINATOR: All crates integrated, server starting successfully"`

## Project Deliverables

### Workspace Structure
```
task-manager/
├── Cargo.toml              # Workspace configuration
├── core/                   # Domain models and traits
├── database/               # SQLite repository implementation
├── mcp-protocol/          # MCP server with SSE
├── mcp-server/            # Main binary
├── mocks/                 # Test utilities
├── PRD.md                 # Product requirements
├── ARCHITECTURE.md        # Detailed architecture
├── TASKLIST.*.md          # Implementation guides per crate
└── .github/workflows/     # CI/CD automation
```

### Key Documentation
- **PRD.md** - Product requirements and MCP function specifications
- **ARCHITECTURE.md** - Multi-crate design with interface contracts
- **TASKLIST files** - Step-by-step implementation guides for each crate
- **Agent specifications** - Role definitions in .claude/agents/

### Quality Assurance
- **Unit Tests**: Per-crate testing with >90% coverage
- **Contract Tests**: Standardized tests for trait implementations
- **Integration Tests**: Full system validation with SSE
- **Mock Testing**: Fast, isolated tests using mock crate

## Success Criteria

### Technical Excellence
- **100% MCP Protocol Compliance**: Full SSE-based MCP implementation
- **Production-Ready Code**: Proper error handling, tracing, graceful shutdown
- **SQLite Integration**: Automatic database path with ~/db.sqlite default
- **Test Coverage**: >90% per crate with contract tests
- **Clean Architecture**: Trait-based design enabling parallel development

### Crate Integration
- **Independent Development**: Each crate compiles and tests standalone
- **Interface Contracts**: All traits defined in core crate
- **Mock Support**: All crates can use mocks for testing
- **Seamless Assembly**: Git-coordinator integrates all components

## MANDATORY EXECUTION PROTOCOL

### CONTROL AGENT EXECUTION REQUIREMENTS

**STEP 1 - IMMEDIATE PARALLEL ACTIVATION**:
```
Use Task tool to launch ALL 5 crate owners simultaneously:
- core-architect (for core crate)
- database-designer (for database crate)  
- mcp-integrator (for mcp-protocol crate)
- integration-lead (for mcp-server crate)
- qa-tester (for mocks crate)
```

**STEP 2 - HANDS-OFF COORDINATION**:
- **DO NOT write any code, documentation, or implementations**
- **DO NOT call ./log.sh pretending to be an agent**
- **DO NOT simulate agent work or responses**
- **ONLY monitor and coordinate if agents request help**

### CRATE OWNER EXECUTION REQUIREMENTS

**IMMEDIATE ACTIONS FOR ALL CRATE OWNERS**:
1. **Read your crate's TASKLIST.[crate-name].md file**
2. **Call `./log.sh "CRATE-OWNER: Starting work on [crate-name] crate"`**
3. **Begin implementing your crate following the TASKLIST**
4. **Coordinate using `./log.sh` when you need interfaces from other crates**
5. **Mark TASKLIST items complete as you progress**
6. **Run `cargo test` frequently to ensure your crate works**

**DEVELOPMENT APPROACH**:
- ✅ Develop against trait interfaces defined in core
- ✅ Use mock implementations for testing
- ✅ Communicate when you need something from another crate
- ✅ Test your crate independently before integration

### Key Technical Requirements
- **SQLite Only**: No PostgreSQL or multi-database support
- **SSE Transport**: MCP communication via Server-Sent Events
- **Automatic DB Path**: Default to ~/db.sqlite when DATABASE_URL not set
- **Error Mapping**: Map all errors to appropriate MCP error codes
- **State Validation**: Enforce valid task state transitions
- **Contract Tests**: Each trait implementation must pass standardized tests

---

**SUCCESS CRITERIA**: All 5 crates developed in parallel, integrated seamlessly, with comprehensive testing and documentation. The final MCP server must handle all 8 required functions via SSE with SQLite persistence.