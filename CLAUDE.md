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

## 🚨 CRITICAL DEVELOPMENT RULES 🚨

**1. DEPENDENCY MANAGEMENT - USE CARGO COMMANDS ONLY**
- ❌ **NEVER** manually edit Cargo.toml files
- ✅ **ALWAYS** use cargo commands to add dependencies
- This ensures latest compatible versions and proper feature flags

Examples:
```bash
cargo add serde --features derive        # Add with features
cargo add tokio --features full          # Add tokio with all features
cargo add --dev mockall                  # Add dev dependency
cargo add core --path ../core            # Add workspace dependency
```

**2. AGGRESSIVE PARALLELIZATION**
- Phase 2 MUST launch 4 agents SIMULTANEOUSLY
- Use ONE Task tool call for all parallel agents
- Sequential launches are FORBIDDEN in parallel phases

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

The project is structured as a Rust workspace with 5 crates with clear dependencies:

### Crate Dependency Graph
```
core (no dependencies - base layer)
  ├── database (depends on core)
  ├── mcp-protocol (depends on core)
  └── mocks (depends on core)
      │
      └── mcp-server (depends on core, database, mcp-protocol)
```

### Crate Ownership (Phase 1 - Core Development)

**1. `core` crate** - Owner: rust-architect
   - Domain models, business logic, trait interfaces
   - Task struct, TaskState enum, error types
   - Repository and protocol handler traits

**2. `database` crate** - Owner: database-engineer
   - SQLite implementation of TaskRepository trait
   - Database migrations and schema management
   - Connection pooling and error mapping

**3. `mcp-protocol` crate** - Owner: protocol-specialist
   - MCP server implementation with SSE transport
   - Protocol handler implementation
   - JSON-RPC message handling

**4. `mcp-server` crate** - Owner: integration-lead
   - Main binary assembling all components
   - Configuration management
   - Dependency injection and startup logic

**5. `mocks` crate** - Owner: testing-expert
   - Mock implementations for testing
   - Test fixtures and generators
   - Contract test helpers

### Supporting Team Members (Phase 2 - Documentation & Finalization)

**6. documentation-specialist** - Creates comprehensive documentation
   - README files for all crates
   - API.md with complete MCP reference
   - User guides and examples
   - Rustdoc for all public APIs

**7. project-finalizer** - Ensures production readiness
   - Final integration testing
   - Performance validation
   - Security audit
   - Cleanup and release preparation

## Development Methodology

### Dependency-Aware Development Process

**Why This Order?**
- `core` must be completed first as all other crates depend on its traits
- `database`, `mcp-protocol`, and `mocks` can be developed in parallel
- `mcp-server` assembles all components, so needs others to be ready
- Documentation comes after implementation
- Finalization ensures production readiness

**Development Principles**
- Core defines stable trait interfaces early
- Other crates develop against these interfaces
- Use mock implementations for testing
- Regular commits of working code
- Communication via ./log.sh

### MANDATORY Execution Rules for Control Agent

**CONTROL AGENT REQUIREMENTS**:
- **Phase 1**: Launch ONLY rust-architect first (WAIT for completion)
- **Phase 2**: ⚠️ IMMEDIATELY launch ALL 4 agents SIMULTANEOUSLY
  - Use ONE Task invocation with all 4 agents
  - DO NOT launch them one by one
  - DO NOT wait between launches
- **Phase 3**: Launch documentation-specialist when Phase 2 is ready
- **Phase 4**: Launch project-finalizer for final validation
- **DO NOT DO ANY WORK YOURSELF** - you only coordinate
- **NO AGENT SIMULATION** - do not pretend to be any agent
- **LET AGENTS COMMUNICATE DIRECTLY** - do not mediate

**PARALLELIZATION ENFORCEMENT**:
```python
# CORRECT - Launch all Phase 2 agents at once:
Task(agents=["database-engineer", "protocol-specialist", "testing-expert", "integration-lead"])

# WRONG - Sequential launches:
Task(agent="database-engineer")
Task(agent="protocol-specialist")  # NO! This wastes time!
```

### Development Requirements for All Agents

**ALL AGENT REQUIREMENTS**:
- **USE CARGO COMMANDS**: NEVER manually edit Cargo.toml files
  - Use `cargo add <crate> --features <features>` for dependencies
  - Use `cargo add <crate> --dev` for dev dependencies
  - This ensures latest compatible versions
- **USE TEMPORARY DIRECTORIES**: Create `./tmp/` in your work area
- **GITIGNORE TEMP FILES**: Ensure ./tmp/ is in .gitignore
- **SELECTIVE COMMITS**: NEVER use `git add .` or `git add -A`
- **REVIEW BEFORE COMMIT**: Always run `git status` and review
- **CLEAN COMMITS**: Remove all temp files before committing
- **COMMIT YOUR WORK**: Commit completed work with clear messages

### Cargo Command Examples

```bash
# Add dependencies with features
cargo add serde --features derive
cargo add tokio --features full
cargo add sqlx --features runtime-tokio-rustls,sqlite,migrate

# Add dev dependencies
cargo add --dev tokio-test
cargo add --dev proptest

# Create new crate in workspace
cargo new --lib core
cargo new --lib database

# Add workspace dependency
cargo add core --path ../core
```

### Communication Requirements
**ALL team members MUST use `./log.sh "message"` for coordination logging**

Examples:
- `./log.sh "RUST-ARCHITECT → DATABASE-ENGINEER: TaskRepository trait defined, ready for implementation"`
- `./log.sh "DATABASE-ENGINEER → TESTING-EXPERT: Need mock repository for testing"`
- `./log.sh "PROTOCOL-SPECIALIST → ALL: SSE endpoint ready at /mcp/v1"`
- `./log.sh "INTEGRATION-LEAD: All crates integrated, server starting successfully"`
- `./log.sh "DOCUMENTATION: API documentation complete, ready for review"`
- `./log.sh "FINALIZER → ALL: Found performance issue in task listing, needs optimization"`

### Phase Completion Protocol
**CRITICAL**: Agents MUST report phase completion to enable transitions:

```bash
# Phase 1 completion
./log.sh "PHASE_1_COMPLETE: core crate ready with all traits defined"

# Phase 2 completion (each agent reports)
./log.sh "PHASE_2_COMPLETE: database crate ready, all tests passing"
./log.sh "PHASE_2_COMPLETE: mcp-protocol crate ready, SSE working"
./log.sh "PHASE_2_COMPLETE: mocks crate ready, all fixtures created"
./log.sh "PHASE_2_COMPLETE: mcp-server skeleton ready, awaiting integration"

# Phase 3 completion
./log.sh "PHASE_3_COMPLETE: documentation complete, all READMEs updated"

# Phase 4 completion
./log.sh "PHASE_4_COMPLETE: production ready, all quality gates passed"
```

### Git Commit Best Practices

**CRITICAL**: All agents must follow these rules:

1. **Check Status First**
   ```bash
   git status  # See what's changed
   git diff    # Review changes
   ```

2. **Add Files Selectively**
   ```bash
   # GOOD - Add specific files
   git add src/models.rs
   git add src/handlers.rs
   
   # BAD - Never do this
   git add .
   git add -A
   ```

3. **Clean Before Commit**
   ```bash
   # Remove temp files
   rm -rf ./tmp/
   find . -name "*.tmp" -delete
   find . -name "*.log" -delete
   ```

4. **Meaningful Commit Messages**
   ```bash
   git commit -m "feat(core): Add Task and TaskState models"
   git commit -m "fix(database): Handle connection timeouts"
   git commit -m "docs: Add API reference for all MCP functions"
   ```

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

### Optimized Four-Phase Execution Plan

**PHASE 1 - Core Foundation (Sequential)**

Launch ONLY the core architect first:
```
- rust-architect (for core crate)
```

Wait for core crate to define all traits and domain models.
This is the foundation that all other crates depend on.

**Definition of Done - Phase 1**:
- ✅ Core crate created with proper structure
- ✅ All traits defined (TaskRepository, ProtocolHandler)
- ✅ All domain models defined (Task, TaskState, errors)
- ✅ Crate compiles without errors
- ✅ Basic documentation in place

**PHASE 2 - Parallel Development**

⚠️ **MANDATORY**: Once core traits are defined, you MUST launch ALL 4 crate owners AT THE SAME TIME:
```
LAUNCH SIMULTANEOUSLY - NO DELAYS:
- database-engineer (for database crate - depends on core)
- protocol-specialist (for mcp-protocol crate - depends on core)  
- testing-expert (for mocks crate - depends on core)
- integration-lead (for mcp-server crate skeleton)
```

**PARALLELIZATION IS CRITICAL**: These crates can and MUST work in parallel!
- They only depend on core traits, not on each other
- Launching them sequentially wastes time and defeats the architecture
- Use a single Task tool invocation to launch all 4 agents
- DO NOT wait for one to finish before launching another

**Definition of Done - Phase 2**:
- ✅ All 4 crates compile independently
- ✅ All unit tests pass in each crate
- ✅ Database migrations work (database crate)
- ✅ SSE endpoint responds (mcp-protocol crate)
- ✅ Mock implementations complete (mocks crate)
- ✅ Server binary starts (mcp-server crate)

**PHASE 3 - Documentation**

When Phase 2 crates are substantially complete, launch:
```
- documentation-specialist
```

Creates comprehensive documentation for all implemented functionality.

**Definition of Done - Phase 3**:
- ✅ All public APIs have rustdoc comments
- ✅ README.md exists for each crate
- ✅ Main README.md comprehensive
- ✅ API.md with all MCP functions documented
- ✅ User guide with examples
- ✅ All documentation examples compile and run

**PHASE 4 - Finalization**

When documentation is complete, launch:
```
- project-finalizer
```

Final integration, testing, and production readiness verification.

**Definition of Done - Phase 4**:
- ✅ All crates build without warnings
- ✅ All tests pass (unit, integration, E2E)
- ✅ Production build optimized
- ✅ Docker image builds and runs
- ✅ No development artifacts remain
- ✅ Clean clone builds and runs successfully
- ✅ Performance benchmarks acceptable
- ✅ Security best practices verified

### CONTROL AGENT EXECUTION REQUIREMENTS

**HANDS-OFF COORDINATION**:
- **DO NOT write any code, documentation, or implementations**
- **DO NOT call ./log.sh pretending to be an agent**
- **DO NOT simulate agent work or responses**
- **ONLY monitor and coordinate between phases**
- **Wait for agents to report completion before next phase**

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

### Temporary File Management

**ALL AGENTS MUST**:
1. Create `./tmp/` directory in their work area
2. Add `**/tmp/` to .gitignore (if not already present)
3. Use ./tmp/ for all temporary files, test outputs, scripts
4. Clean ./tmp/ before any commit
5. Never commit temporary or test files

Example .gitignore entries:
```
**/tmp/
**/*.tmp
**/*.log
**/*.bak
**/target/
```

---

**SUCCESS CRITERIA**: 
- Phase 1: Core crate complete with all traits and domain models
- Phase 2: Database, mcp-protocol, mocks crates working; mcp-server integrated
- Phase 3: Complete documentation with examples and guides
- Phase 4: Production-ready system with all quality gates passed
- Final: Clean repository with no development artifacts

**DEPENDENCY VALIDATION**:
- No crate should have compilation errors due to missing dependencies
- All trait implementations must satisfy contract tests
- Integration must be seamless with proper error handling