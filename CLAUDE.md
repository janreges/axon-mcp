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

Develop a production-ready Model Context Protocol (MCP) server in Rust for comprehensive task management and workflow coordination. This MCP will serve as a critical infrastructure component for AI agent teams, providing robust task tracking, assignment, and lifecycle management capabilities.

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
- **Database**: SQLite for embedded deployment, PostgreSQL-ready architecture
- **Serialization**: JSON for MCP protocol compliance
- **Testing**: Comprehensive unit, integration, and performance tests
- **Documentation**: Full API documentation with examples

## Team Structure

### 8-Person Senior Development Team

**1. rust-architect** - System design, architecture decisions, technical leadership
**2. backend-developer** - Core Rust implementation, business logic, performance optimization
**3. database-designer** - Data modeling, schema design, query optimization
**4. mcp-integrator** - MCP protocol implementation, client-server communication
**5. documentation-specialist** - Technical documentation, API guides, code examples
**6. qa-tester** - Test strategy, quality assurance, performance validation
**7. git-coordinator** - Version control, branching strategy, code integration
**8. devops-engineer** - Build systems, CI/CD, deployment automation

## Development Methodology

### MANDATORY Aggressive Parallel Development

**CONTROL AGENT REQUIREMENTS**:
- **IMMEDIATELY launch ALL 8 agents using Task tool** - no delays, no sequential activation
- **DO NOT DO ANY WORK YOURSELF** - you only coordinate, never implement
- **NO AGENT SIMULATION** - do not pretend to be any agent or use their log.sh
- **LET AGENTS COMMUNICATE DIRECTLY** - do not mediate their conversations

**AGENT REQUIREMENTS**:
- **START IMMEDIATELY**: Begin work the moment you are activated
- **COMMUNICATE DIRECTLY**: Use ./log.sh to coordinate with other agents by name
- **WORK IN PARALLEL**: Do not wait for others to finish - work simultaneously
- **CROSS-FUNCTIONAL COLLABORATION**: Help teammates beyond your primary role when needed

### Communication Requirements
**ALL team members MUST use `./log.sh "message"` for coordination logging**

Examples:
- `./log.sh "RUST-ARCHITECT → DATABASE: Need schema for task relationships"`
- `./log.sh "BACKEND → MCP-INTEGRATOR: Core task struct ready for protocol mapping"`
- `./log.sh "QA → ALL: Found critical issue in task state transitions"`

## Project Deliverables

### Code Architecture
- `src/main.rs` - MCP server entry point and configuration
- `src/lib.rs` - Core library exports and module organization
- `src/models/` - Task data structures and business logic
- `src/handlers/` - MCP function implementations
- `src/database/` - Database abstraction and query implementation
- `src/error.rs` - Comprehensive error handling
- `src/config.rs` - Configuration management

### Supporting Infrastructure  
- `Cargo.toml` - Project dependencies and metadata
- `README.md` - Project overview and setup instructions
- `API.md` - Complete MCP function documentation
- `ARCHITECTURE.md` - System design and technical decisions
- `tests/` - Comprehensive test suite
- `.github/workflows/` - CI/CD automation

### Quality Assurance
- **Unit Tests**: Individual component testing
- **Integration Tests**: End-to-end MCP functionality
- **Performance Tests**: Concurrent load and scalability validation
- **Documentation Tests**: Code example verification

## Success Criteria

### Technical Excellence
- **100% MCP Protocol Compliance**: Full adherence to MCP specification
- **Production-Ready Code**: Error handling, logging, configuration management
- **Comprehensive Documentation**: API guides, architecture documentation, usage examples
- **Test Coverage**: >90% code coverage with meaningful test scenarios

### Team Collaboration
- **Parallel Development**: All team members contributing simultaneously
- **Cross-Functional Integration**: Seamless collaboration across specializations
- **Continuous Delivery**: Regular commits and progressive feature development
- **Quality Maintenance**: High code quality despite aggressive development pace

## MANDATORY EXECUTION PROTOCOL

### CONTROL AGENT EXECUTION REQUIREMENTS

**STEP 1 - IMMEDIATE PARALLEL ACTIVATION**:
```
Use Task tool to launch ALL 8 agents simultaneously:
- rust-architect
- backend-developer  
- database-designer
- mcp-integrator  
- documentation-specialist
- qa-tester
- git-coordinator
- devops-engineer
```

**STEP 2 - HANDS-OFF COORDINATION**:
- **DO NOT write any code, documentation, or implementations**
- **DO NOT call ./log.sh pretending to be an agent**
- **DO NOT simulate agent work or responses**
- **ONLY monitor and coordinate if agents request help**

### AGENT EXECUTION REQUIREMENTS

**IMMEDIATE ACTIONS FOR ALL AGENTS**:
1. **Call `./log.sh "AGENT-NAME: Starting parallel work on MCP project"`**
2. **Begin your specialized work immediately without waiting**
3. **Coordinate with other agents using `./log.sh "AGENT-NAME → TARGET: message"`**
4. **Create, modify, and commit code files as needed**
5. **Ask other agents for help or coordination when needed**

**FORBIDDEN BEHAVIORS**:
- ❌ Waiting for other agents to complete their work before starting
- ❌ Working in isolation without communicating with teammates
- ❌ Assuming what other agents will do instead of asking them directly

### Code Quality Standards
- **Rust Best Practices**: Idiomatic Rust code with proper error handling
- **Performance Optimization**: Efficient algorithms and minimal resource usage
- **Security Considerations**: Input validation, safe concurrency, audit logging  
- **Maintainability**: Clear code structure, comprehensive documentation

---

**SUCCESS CRITERIA**: All 8 agents working simultaneously, communicating directly, and producing a complete professional MCP system with comprehensive documentation, tests, and deployment automation.