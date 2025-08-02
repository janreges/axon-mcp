# MCP Task Management Server - API Reference

This document provides comprehensive documentation for all Model Context Protocol (MCP) functions implemented by the Task Management Server.

## Protocol Overview

The server implements the MCP specification using:
- **Transport**: Server-Sent Events (SSE) over HTTP
- **Protocol**: JSON-RPC 2.0 for message exchange
- **Endpoint**: `/mcp/v1` (default)

## Connection

### Establishing Connection

Connect to the MCP server using SSE:

```javascript
const eventSource = new EventSource('http://localhost:3000/mcp/v1');

eventSource.onmessage = function(event) {
    const response = JSON.parse(event.data);
    console.log('Received:', response);
};
```

### Request Format

All requests follow JSON-RPC 2.0 format:

```json
{
    "jsonrpc": "2.0",
    "id": "unique-request-id",
    "method": "function_name",
    "params": {
        // Function-specific parameters
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
        // Function result data
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
        "message": "Error description",
        "data": {
            // Additional error context
        }
    }
}
```

## Data Models

### Task Object

```json
{
    "id": 123,
    "code": "ARCH-001",
    "name": "Design system architecture",
    "description": "Create detailed architecture for the MCP task management system",
    "owner_agent_name": "rust-architect",
    "state": "InProgress",
    "inserted_at": "2025-01-29T10:30:00Z",
    "done_at": null
}
```

### Task States

- `"Created"` - Initial state for new tasks
- `"InProgress"` - Task actively being worked on
- `"Blocked"` - Task temporarily blocked by dependencies
- `"Review"` - Task awaiting review or approval  
- `"Done"` - Task successfully completed
- `"Archived"` - Task moved to long-term storage
- `"PendingDecomposition"` - Task needs to be broken down into subtasks
- `"PendingHandoff"` - Waiting for agent handoff
- `"Quarantined"` - Too many failures, needs human review
- `"WaitingForDependency"` - Blocked on other tasks completing

### Error Codes

| Code | Name | Description |
|------|------|-------------|
| -32001 | TaskNotFound | Task with specified ID or code not found |
| -32002 | ValidationError | Input validation failed |
| -32003 | DuplicateCode | Task code already exists |
| -32004 | InvalidStateTransition | Invalid state change attempted |
| -32005 | DatabaseError | Database operation failed |
| -32006 | ProtocolError | MCP protocol error |
| -32007 | SerializationError | JSON serialization/deserialization error |

## MCP Functions

### create_task

Creates a new task with validation.

**Parameters:**
- `code` (string, required): Unique human-readable identifier
- `name` (string, required): Brief task title  
- `description` (string, required): Detailed task requirements
- `owner_agent_name` (string, optional): Agent identifier (null for unassigned tasks)

**Returns:** Complete Task object with generated ID and timestamps

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-001",
    "method": "create_task",
    "params": {
        "code": "FEAT-042",
        "name": "Implement user authentication",
        "description": "Add JWT-based authentication with role-based access control",
        "owner_agent_name": "backend-developer"
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-001",
    "result": {
        "id": 42,
        "code": "FEAT-042",
        "name": "Implement user authentication",
        "description": "Add JWT-based authentication with role-based access control",
        "owner_agent_name": "backend-developer",
        "state": "Created",
        "inserted_at": "2025-01-29T14:30:15Z",
        "done_at": null
    }
}
```

**Errors:**
- `DuplicateCode`: Task code already exists
- `ValidationError`: Missing or invalid parameters

---

### update_task

Updates an existing task's metadata.

**Parameters:**
- `id` (integer, required): Numeric task identifier
- `name` (string, optional): New task title
- `description` (string, optional): New task description

**Returns:** Updated Task object

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-002",
    "method": "update_task", 
    "params": {
        "id": 42,
        "name": "Implement JWT authentication system",
        "description": "Add JWT-based authentication with role-based access control and session management"
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-002",
    "result": {
        "id": 42,
        "code": "FEAT-042",
        "name": "Implement JWT authentication system",
        "description": "Add JWT-based authentication with role-based access control and session management",
        "owner_agent_name": "backend-developer",
        "state": "Created",
        "inserted_at": "2025-01-29T14:30:15Z",
        "done_at": null
    }
}
```

**Errors:**
- `TaskNotFound`: Task with specified ID does not exist
- `ValidationError`: Invalid parameter values

---

### set_task_state

Changes a task's lifecycle state with validation.

**Parameters:**
- `id` (integer, required): Numeric task identifier
- `state` (string, required): Target state ("Created", "InProgress", "Blocked", "Review", "Done", "Archived", "PendingDecomposition", "PendingHandoff", "Quarantined", "WaitingForDependency")

**Returns:** Updated Task object with new state

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-003",
    "method": "set_task_state",
    "params": {
        "id": 42,
        "state": "InProgress"
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-003",
    "result": {
        "id": 42,
        "code": "FEAT-042",
        "name": "Implement JWT authentication system",
        "description": "Add JWT-based authentication with role-based access control and session management",
        "owner_agent_name": "backend-developer",
        "state": "InProgress",
        "inserted_at": "2025-01-29T14:30:15Z",
        "done_at": null
    }
}
```

**State Transition Rules:**
- `Created` → `InProgress`, `PendingDecomposition`, `WaitingForDependency`
- `InProgress` → `Blocked`, `Review`, `Done`, `PendingHandoff`
- `Blocked` → `InProgress`
- `Review` → `InProgress`, `Done`
- `Done` → `Archived` (via archive_task only)
- `PendingDecomposition` → `Created` (after decomposition)
- `PendingHandoff` → `InProgress` (when handoff accepted)
- `Quarantined` → `Created` (after human review)
- `WaitingForDependency` → `Created` (when dependencies met)
- `Archived` → (no transitions allowed)
- Any state → `Quarantined` (emergency quarantine)

**Errors:**
- `TaskNotFound`: Task with specified ID does not exist
- `InvalidStateTransition`: Invalid state change attempted

---

### get_task_by_id

Retrieves a single task by numeric ID.

**Parameters:**
- `id` (integer, required): Numeric task identifier

**Returns:** Task object or null if not found

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-004",
    "method": "get_task_by_id",
    "params": {
        "id": 42
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-004",
    "result": {
        "id": 42,
        "code": "FEAT-042",
        "name": "Implement JWT authentication system",
        "description": "Add JWT-based authentication with role-based access control and session management",
        "owner_agent_name": "backend-developer",
        "state": "InProgress",
        "inserted_at": "2025-01-29T14:30:15Z",
        "done_at": null
    }
}
```

**Errors:**
- `TaskNotFound`: Task with specified ID does not exist

---

### get_task_by_code

Retrieves a single task by human-readable code.

**Parameters:**
- `code` (string, required): Human-readable task identifier

**Returns:** Task object or null if not found

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-005",
    "method": "get_task_by_code",
    "params": {
        "code": "FEAT-042"
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-005", 
    "result": {
        "id": 42,
        "code": "FEAT-042",
        "name": "Implement JWT authentication system",
        "description": "Add JWT-based authentication with role-based access control and session management",
        "owner_agent_name": "backend-developer",
        "state": "InProgress",
        "inserted_at": "2025-01-29T14:30:15Z",
        "done_at": null
    }
}
```

**Errors:**
- `TaskNotFound`: Task with specified code does not exist

---

### list_tasks

Queries tasks with optional filters.

**Parameters (all optional):**
- `owner` (string): Filter by agent name
- `state` (string): Filter by task state
- `date_from` (string, ISO 8601): Filter tasks created after this date
- `date_to` (string, ISO 8601): Filter tasks created before this date

**Returns:** Array of Task objects matching criteria

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-006",
    "method": "list_tasks",
    "params": {
        "owner": "backend-developer",
        "state": "InProgress"
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-006",
    "result": [
        {
            "id": 42,
            "code": "FEAT-042",
            "name": "Implement JWT authentication system",
            "description": "Add JWT-based authentication with role-based access control and session management",
            "owner_agent_name": "backend-developer",
            "state": "InProgress",
            "inserted_at": "2025-01-29T14:30:15Z",
            "done_at": null
        },
        {
            "id": 43,
            "code": "API-001",
            "name": "Design REST API endpoints",
            "description": "Create OpenAPI specification for all endpoints",
            "owner_agent_name": "backend-developer",
            "state": "InProgress", 
            "inserted_at": "2025-01-29T15:45:22Z",
            "done_at": null
        }
    ]
}
```

**Errors:**
- `ValidationError`: Invalid filter parameters

---

### assign_task

Transfers task ownership to another agent.

**Parameters:**
- `id` (integer, required): Numeric task identifier
- `new_owner_agent_name` (string, required): Target agent identifier

**Returns:** Updated Task object with new owner

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-007",
    "method": "assign_task",
    "params": {
        "id": 42,
        "new_owner_agent_name": "frontend-developer"
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-007",
    "result": {
        "id": 42,
        "code": "FEAT-042",
        "name": "Implement JWT authentication system",
        "description": "Add JWT-based authentication with role-based access control and session management",
        "owner_agent_name": "frontend-developer",
        "state": "InProgress",
        "inserted_at": "2025-01-29T14:30:15Z",
        "done_at": null
    }
}
```

**Errors:**
- `TaskNotFound`: Task with specified ID does not exist
- `ValidationError`: Invalid agent name

---

### archive_task

Moves a task to archived state with audit trail.

**Parameters:**
- `id` (integer, required): Numeric task identifier

**Returns:** Archived Task object with done_at timestamp

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-008",
    "method": "archive_task",
    "params": {
        "id": 42
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-008",
    "result": {
        "id": 42,
        "code": "FEAT-042",
        "name": "Implement JWT authentication system",
        "description": "Add JWT-based authentication with role-based access control and session management",
        "owner_agent_name": "frontend-developer",
        "state": "Archived",
        "inserted_at": "2025-01-29T14:30:15Z",
        "done_at": "2025-01-30T09:15:33Z"
    }
}
```

**Prerequisites:**
- Task must be in "Done" state before archiving

**Errors:**
- `TaskNotFound`: Task with specified ID does not exist  
- `InvalidStateTransition`: Task not in "Done" state

### health_check

Checks server health and status.

**Parameters:** None

**Returns:** HealthStatus object with system information

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-009",
    "method": "health_check",
    "params": {}
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-009",
    "result": {
        "status": "healthy",
        "uptime": "2h 15m 30s",
        "database_connected": true,
        "total_tasks": 1247,
        "active_agents": 12
    }
}
```

## MCP v2 Advanced Multi-Agent Operations

### discover_work

Finds available tasks based on agent capabilities.

**Parameters:**
- `agent_name` (string, required): Agent identifier
- `capabilities` (array of strings, required): Agent skills/technologies
- `max_tasks` (integer, optional): Maximum number of tasks to return (default: 10)

**Returns:** Array of available Task objects

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-010",
    "method": "discover_work",
    "params": {
        "agent_name": "python-specialist",
        "capabilities": ["python", "fastapi", "postgresql"],
        "max_tasks": 5
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-010",
    "result": [
        {
            "id": 123,
            "code": "API-001",
            "name": "Build user authentication API",
            "description": "Implement FastAPI endpoints for user login/logout with PostgreSQL backend",
            "owner_agent_name": null,
            "state": "Created",
            "inserted_at": "2025-01-29T10:30:00Z",
            "done_at": null
        }
    ]
}
```

### claim_task

Atomically claims a task for execution by an agent.

**Parameters:**
- `task_id` (integer, required): Numeric task identifier
- `agent_name` (string, required): Agent claiming the task (kebab-case format)

**Returns:** Task object with state set to "InProgress"

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-011",
    "method": "claim_task",
    "params": {
        "task_id": 123,
        "agent_name": "python-specialist"
    }
}
```

**Example Response:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-011",
    "result": {
        "id": 123,
        "code": "API-001",
        "name": "Build user authentication API",
        "description": "Implement FastAPI endpoints for user login/logout with PostgreSQL backend",
        "owner_agent_name": "python-specialist",
        "state": "InProgress",
        "inserted_at": "2025-01-29T10:30:00Z",
        "done_at": null
    }
}
```

**Errors:**
- `TaskNotFound`: Task with specified ID does not exist
- `ValidationError`: Agent name format invalid (must be kebab-case)
- `InvalidStateTransition`: Task already claimed or not in "Created" state

### release_task

Releases a previously claimed task back to the available pool.

**Parameters:**
- `task_id` (integer, required): Numeric task identifier
- `agent_name` (string, required): Agent releasing the task

**Returns:** Task object with state reset to "Created"

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-012",
    "method": "release_task",
    "params": {
        "task_id": 123,
        "agent_name": "python-specialist"
    }
}
```

### start_work_session

Begins time tracking for task work.

**Parameters:**
- `task_id` (integer, required): Numeric task identifier
- `agent_name` (string, required): Agent starting work
- `description` (string, optional): Work session description

**Returns:** WorkSessionInfo object with session details

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-013",
    "method": "start_work_session",
    "params": {
        "task_id": 123,
        "agent_name": "python-specialist",
        "description": "Starting API implementation"
    }
}
```

### end_work_session

Ends time tracking for task work.

**Parameters:**
- `session_id` (string, required): Work session identifier
- `summary` (string, optional): Work completed summary

**Returns:** Empty success response

## Inter-Agent Messaging

### create_task_message

Creates a message within a task context for agent communication.

**Parameters:**
- `task_code` (string, required): Human-readable task identifier
- `author_agent_name` (string, required): Agent sending the message
- `target_agent_name` (string, optional): Specific recipient agent
- `message_type` (string, required): Message type ("handoff", "comment", "question", "solution", "blocker")
- `content` (string, required): Message content
- `reply_to_message_id` (integer, optional): For threading conversations

**Returns:** TaskMessage object

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-014",
    "method": "create_task_message",
    "params": {
        "task_code": "API-001",
        "author_agent_name": "frontend-developer",
        "target_agent_name": "backend-developer",
        "message_type": "handoff",
        "content": "Frontend auth components ready. Need /login and /logout endpoints with JWT tokens."
    }
}
```

### get_task_messages

Retrieves messages for a task with optional filtering.

**Parameters:**
- `task_code` (string, required): Human-readable task identifier
- `author_agent_name` (string, optional): Filter by message author
- `target_agent_name` (string, optional): Filter by message recipient
- `message_type` (string, optional): Filter by message type
- `reply_to_message_id` (integer, optional): Get conversation thread
- `limit` (integer, optional): Maximum messages to return

**Returns:** Array of TaskMessage objects

**Example Request:**
```json
{
    "jsonrpc": "2.0",
    "id": "req-015",
    "method": "get_task_messages",
    "params": {
        "task_code": "API-001",
        "target_agent_name": "backend-developer",
        "message_type": "handoff"
    }
}
```

## Workspace Setup Automation

### get_setup_instructions

Generates AI workspace setup instructions based on PRD analysis.

**Parameters:**
- `prd_content` (string, required): Product Requirements Document content
- `ai_tool_type` (string, required): AI tool type ("claude-code")

**Returns:** SetupInstructions object with step-by-step guidance

### get_agentic_workflow_description

Analyzes PRD and recommends optimal agent roles and workflow.

**Parameters:**
- `prd_content` (string, required): Product Requirements Document content

**Returns:** AgenticWorkflowDescription with recommended agents

### register_agent

Registers an AI agent in the workspace.

**Parameters:**
- `agent_name` (string, required): Agent identifier (kebab-case)
- `capabilities` (array of strings, required): Agent skills
- `role_description` (string, required): Agent's role and responsibilities

**Returns:** AgentRegistration confirmation

### get_instructions_for_main_ai_file

Gets instructions for creating the main AI coordination file.

**Parameters:**
- `project_summary` (string, required): Brief project description
- `ai_tool_type` (string, required): AI tool type ("claude-code")

**Returns:** MainAiFileInstructions with file content guidelines

### create_main_ai_file

Creates the main AI coordination file (CLAUDE.md, etc.).

**Parameters:**
- `project_summary` (string, required): Brief project description
- `ai_tool_type` (string, required): AI tool type ("claude-code")
- `additional_context` (string, optional): Extra project context

**Returns:** MainAiFileData with generated file content

### get_workspace_manifest

Generates complete workspace manifest for AI automation.

**Parameters:**
- `prd_content` (string, required): Product Requirements Document content

**Returns:** WorkspaceManifest with full workspace configuration

## Usage Examples

### Complete Task Workflow

```javascript
// 1. Create a task
const createResponse = await sendMCPRequest({
    method: "create_task",
    params: {
        code: "BUG-001",
        name: "Fix login timeout issue",
        description: "Users report login timeouts after 30 seconds",
        owner_agent_name: "bug-hunter"
    }
});

const taskId = createResponse.result.id;

// 2. Start working on it
await sendMCPRequest({
    method: "set_task_state",
    params: {
        id: taskId,
        state: "InProgress"
    }
});

// 3. Update details as needed
await sendMCPRequest({
    method: "update_task",
    params: {
        id: taskId,
        description: "Login timeout caused by database connection pool exhaustion. Fixed by increasing pool size and adding proper connection recycling."
    }
});

// 4. Complete the task
await sendMCPRequest({
    method: "set_task_state", 
    params: {
        id: taskId,
        state: "Done"
    }
});

// 5. Archive when ready
await sendMCPRequest({
    method: "archive_task",
    params: {
        id: taskId
    }
});
```

### Querying Tasks by Agent

```javascript
// Get all tasks for a specific agent
const agentTasks = await sendMCPRequest({
    method: "list_tasks",
    params: {
        owner: "bug-hunter"
    }
});

// Get all blocked tasks
const blockedTasks = await sendMCPRequest({
    method: "list_tasks", 
    params: {
        state: "Blocked"
    }
});

// Get tasks from the last week
const recentTasks = await sendMCPRequest({
    method: "list_tasks",
    params: {
        date_from: "2025-01-22T00:00:00Z",
        date_to: "2025-01-29T23:59:59Z"
    }
});
```

## Performance Considerations

- **Response Time**: All operations complete in <100ms under normal load
- **Throughput**: Server handles >1000 operations per second
- **Connection Limits**: Supports 100+ concurrent SSE connections
- **Database**: Optimized for 1M+ tasks with proper indexing

## Rate Limiting

Currently no rate limiting is implemented. Future versions will include:
- Per-agent rate limits
- Global server rate limits  
- Burst protection

## Security

- **Input Validation**: All parameters validated before processing
- **SQL Injection Prevention**: Parameterized queries only
- **Error Information**: Error messages don't leak sensitive data

*Note: This server is designed for local development use and does not include authentication mechanisms.*

## Monitoring and Debugging

Enable debug logging:
```bash
RUST_LOG=debug ./mcp-server
```

Server metrics are available through:
- Health check endpoint: `/health`
- Metrics endpoint: `/metrics` (Prometheus format)
- Connection status in server logs