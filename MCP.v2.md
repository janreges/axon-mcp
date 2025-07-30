# MCP v2: Advanced Multi-Agent Task Management Server

## Executive Summary

Building on the basic CRUD foundation of MCP v1, this specification defines a sophisticated Model Context Protocol server that enables seamless multi-agent collaboration on complex projects. The system goes beyond simple task tracking to provide intelligent workflow orchestration, knowledge transfer, and self-organizing agent coordination.

## Core Vision

**The Problem**: Multiple specialized AI agents working on interconnected tasks need a way to:
- Seamlessly hand off work between agents with full context preservation
- Automatically discover and claim appropriate work based on capabilities
- Maintain project-wide visibility and coordination
- Learn from collective progress and failures

**The Solution**: A hybrid pull-based system where agents autonomously discover prioritized work while the system maintains global optimization and knowledge continuity.

## Technical Architecture

### Enhanced Data Models

#### Core Task Model (Extended)
```rust
struct Task {
    // Basic fields (from v1)
    id: i32,
    code: String,                    // Auto-generated from prefix (e.g., "SEC-001", "ARCH-015")
    name: String,
    description: String,
    owner_agent_name: Option<String>, // kebab-case agent name
    state: TaskState,
    inserted_at: DateTime<Utc>,
    done_at: Option<DateTime<Utc>>,
    
    // Enhanced fields (v2)
    workflow_definition_id: Option<i32>,
    workflow_cursor: Option<String>,     // Current step ID in workflow
    priority_score: f64,                 // Dynamic priority (0.0-1.0)
    parent_task_id: Option<i32>,         // For hierarchical decomposition
    failure_count: i32,                  // Circuit breaker mechanism
    required_capabilities: Vec<String>,   // What skills this task needs
    estimated_effort: Option<i32>,       // Time estimate in minutes
    confidence_threshold: f64,           // Minimum confidence required for handoff
}

enum TaskState {
    // Basic states (from v1)
    Created,
    InProgress, 
    Blocked,
    Review,
    Done,
    Archived,
    
    // Enhanced states (v2)
    PendingDecomposition,    // Task needs to be broken down
    PendingHandoff,          // Waiting for agent handoff
    Quarantined,             // Too many failures, needs human review
    WaitingForDependency,    // Blocked on other tasks
}
```

#### Knowledge Objects (NEW)
```rust
struct KnowledgeObject {
    id: i32,
    task_code: String,               // task code instead of ID
    author_agent_name: String,       // kebab-case agent name
    knowledge_type: KnowledgeType,
    created_at: DateTime<Utc>,
    title: String,
    body: String,                    // Markdown formatted content
    tags: Vec<String>,               // For filtering and search
    visibility: Visibility,          // Public, team, or private
    parent_knowledge_id: Option<i32>, // For threading conversations
    confidence_score: Option<f64>,   // Agent's confidence in this info
    artifacts: serde_json::Value,    // Links to files, code, etc.
}

// Task Messages/Comments (NEW)
struct TaskMessage {
    id: i32,
    task_code: String,               // task code instead of ID
    author_agent_name: String,       // kebab-case agent name
    message_type: MessageType,
    created_at: DateTime<Utc>,
    content: String,                 // Message content
    reply_to_message_id: Option<i32>, // For threading
}

enum MessageType {
    Comment,         // General comment
    Question,        // Question that needs answering
    Update,          // Status or progress update
    Blocker,         // Issue preventing progress
    Solution,        // Solution or workaround
    Review,          // Code/work review comment
    Handoff,         // Handoff related message
}

enum KnowledgeType {
    Note,              // General observation or comment
    Decision,          // Important decision with rationale
    Question,          // Question that needs answering
    Answer,            // Response to a question
    Handoff,           // Formal handoff package
    StepOutput,        // Output from a workflow step
    Blocker,           // Issue preventing progress
    Resolution,        // Solution to a blocker
    Artifact,          // Reference to external resource
}

enum Visibility {
    Public,    // Visible to all agents
    Team,      // Visible to agents with shared capabilities
    Private,   // Only visible to author and task owner
}
```

#### Agent Registry (NEW)
```rust
struct AgentProfile {
    id: i32,
    name: String,                    // kebab-case format (e.g., "ui-ux-researcher")
    description: String,             // Up to 4000 chars describing agent's work scope
    capabilities: Vec<String>,       // e.g., ["rust", "architecture", "testing"]
    max_concurrent_tasks: i32,
    current_load: i32,               // Number of active tasks
    status: AgentStatus,
    preferences: serde_json::Value,  // Working hours, preferences, etc.
    last_heartbeat: DateTime<Utc>,
    reputation_score: f64,           // Based on task completion quality
    specializations: Vec<String>,    // Deep expertise areas
    registered_at: DateTime<Utc>,
    registered_by: String,           // Who registered this agent
}

enum AgentStatus {
    Idle,          // Available for work
    Active,        // Currently working
    Blocked,       // Stuck on current task
    Unresponsive,  // Missed heartbeats
    Offline,       // Deliberately offline
}
```

#### Workflow Definitions (NEW)
```rust
struct WorkflowDefinition {
    id: i32,
    name: String,
    description: String,
    steps: Vec<WorkflowStep>,
    transitions: serde_json::Value,  // Step transition rules
    created_by: i32,
    is_template: bool,               // Can be reused for similar tasks
}

struct WorkflowStep {
    id: String,                      // Unique within workflow
    name: String,
    required_capability: String,     // What kind of agent can do this
    estimated_duration: Option<i32>, // Minutes
    exit_conditions: Vec<String>,    // When this step is complete
    validation_rules: Vec<String>,   // Quality gates
    handoff_template: Option<String>, // Template for handoff message
}
```

#### Event Log (NEW)
```rust
struct SystemEvent {
    id: i32,
    timestamp: DateTime<Utc>,
    event_type: EventType,
    actor_type: ActorType,
    actor_id: i32,
    task_id: Option<i32>,
    payload: serde_json::Value,
    correlation_id: Option<String>,  // For tracing related events
}

enum EventType {
    // Task lifecycle
    TaskCreated, TaskUpdated, TaskClaimed, TaskReleased, TaskCompleted,
    TaskDecomposed, TaskQuarantined, TaskAdvanced,
    
    // Agent lifecycle  
    AgentRegistered, AgentStatusChanged, AgentHeartbeat,
    
    // Knowledge events
    KnowledgeAdded, HandoffInitiated, HandoffCompleted,
    
    // System events
    WorkflowDefinitionCreated, PriorityRecalculated, SystemAlert,
}

enum ActorType {
    Agent,
    System,
    Human,
}
```

## Error Handling

### Agent Validation
All MCP functions that accept agent names will validate against the registered agent registry. If an unknown agent name is provided, the function returns:

```rust
MCPError::UnknownAgent {
    provided_agent: String,
    known_agents: Vec<String>,  // List of all registered agent names
}
```

Example error response:
```json
{
    "error": "UnknownAgent",
    "message": "Agent 'fronted-dev' is unknown. Known agents are: frontend-developer, backend-developer, database-engineer, ui-ux-researcher"
}
```

### Special Agent Names
- `"all"` - Special reserved name meaning "assign to all registered agents"
- Must not conflict with any registered agent name

## Core MCP Functions

### Enhanced Task Management

#### Basic Operations (Enhanced from v1)
```rust
// Enhanced create with workflow support
create_task(
    code: TaskCode,  // Either prefix for auto-generation or explicit predefined code
    name: String, 
    description: String,
    required_capabilities: Vec<String>,
    workflow_definition_id: Option<i32>,
    parent_task_id: Option<i32>,
    priority_score: Option<f64>
) -> Result<Task, MCPError>

enum TaskCode {
    AutoGenerate(String),  // e.g., "SEC" - system generates "SEC-001", "SEC-002", etc.
    Explicit(String),      // e.g., "SEC04" - use this exact code (analyst predefined)
}

// Bulk task creation with predefined assignments and workflow
create_task_hierarchy(
    parent_task: TaskInput,
    subtasks: Vec<SubtaskWithAssignment>,
    workflow_sequence: Vec<WorkflowStep>
) -> Result<TaskHierarchy, MCPError>

struct SubtaskWithAssignment {
    code: TaskCode,               // Either auto-generate from prefix or use explicit code
    name: String,
    description: String,
    required_capabilities: Vec<String>,
    assigned_agent_name: String,  // e.g., "ui-ux-researcher" or "all" for all agents
    depends_on: Vec<String>,      // Task codes this subtask depends on
    // Note: sequence_order derived from array position during creation
}

// Enhanced listing with sophisticated filtering
list_tasks(
    agent_name: Option<String>,  // kebab-case
    state: Option<TaskState>,
    required_capabilities: Option<Vec<String>>,
    priority_range: Option<(f64, f64)>,
    dependency_of: Option<i32>,
    limit: Option<i32>,
    offset: Option<i32>
) -> Result<Vec<Task>, MCPError>

// Get tasks that an agent can work on (capability matching) with long-polling
discover_work(
    agent_name: String,  // kebab-case
    limit: Option<i32>,
    timeout_seconds: Option<i32>  // Default 120s, max 120s
) -> Result<DiscoverWorkResponse, MCPError>

enum DiscoverWorkResponse {
    TasksAvailable(Vec<Task>),           // Found tasks immediately or within timeout
    NoTasksAvailable,                    // No tasks found within timeout period
    PrerequisiteActionRequired(ActionRequired), // Must complete action before getting tasks
}

struct ActionRequired {
    action_type: String,                 // "answer_question", "share_knowledge", etc.
    message: String,                     // Human-readable instruction
    respond_via_function: String,        // Which MCP function to call
    context: serde_json::Value,          // Additional context data
}

// Atomic task claiming with conflict resolution
claim_task(agent_name: String, task_code: String) -> Result<ClaimResult, MCPError>     // kebab-case
release_task(agent_name: String, task_code: String, reason: String) -> Result<(), MCPError>  // kebab-case
```

#### Advanced Task Operations (NEW)
```rust
// Hierarchical task decomposition
decompose_task(
    parent_task_code: String,
    decomposer_agent_name: String,  // kebab-case
    subtasks: Vec<SubtaskPlan>,
    rationale: String
) -> Result<Vec<Task>, MCPError>

// Workflow advancement with validation
advance_task_workflow(
    task_code: String,
    agent_name: String,  // kebab-case
    output_summary: String,
    confidence_score: f64,
    artifacts: serde_json::Value,
    next_step_guidance: Option<String>
) -> Result<WorkflowAdvanceResult, MCPError>

// Dynamic priority adjustment
update_task_priority(
    task_code: String,
    new_priority: f64,
    reason: String,
    updated_by: String  // kebab-case agent name
) -> Result<(), MCPError>

// Circuit breaker management
quarantine_task(
    task_code: String,
    reason: String,
    quarantined_by: String  // kebab-case agent name
) -> Result<(), MCPError>

// Manual unblocking of quarantined tasks (requires high privileges)
unquarantine_task(
    task_code: String,
    reason: String,
    authorized_by: String,  // kebab-case agent name with admin/manager privileges
    reset_failure_count: bool
) -> Result<(), MCPError>
```

### Automatic Time Tracking

```rust
// Work session management with automatic time tracking
start_working_on_task(
    agent_name: String,  // kebab-case
    task_code: String
) -> Result<WorkSession, MCPError>

finish_working_on_task(
    agent_name: String,  // kebab-case
    task_code: String, 
    completion_notes: String
) -> Result<WorkSession, MCPError>

pause_work_on_task(
    agent_name: String,  // kebab-case
    task_code: String, 
    reason: String
) -> Result<(), MCPError>

resume_work_on_task(
    agent_name: String,  // kebab-case
    task_code: String
) -> Result<(), MCPError>

// Get current work session for an agent
get_active_work_session(agent_name: String) -> Result<Option<WorkSession>, MCPError>  // kebab-case

struct WorkSession {
    id: i32,
    agent_name: String,              // kebab-case agent name
    task_code: String,               // task code instead of ID
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    total_active_minutes: i32,
    interruptions: Vec<WorkInterruption>,
    is_active: bool,
}

struct WorkInterruption {
    paused_at: DateTime<Utc>,
    resumed_at: Option<DateTime<Utc>>,
    reason: String,
    duration_minutes: Option<i32>,
}
```

### Knowledge Management System

```rust
// Core knowledge operations
add_knowledge(
    task_code: String,
    author_agent_name: String,       // kebab-case
    knowledge_type: KnowledgeType,
    title: String,
    body: String,
    tags: Vec<String>,
    visibility: Visibility,
    confidence_score: Option<f64>,
    artifacts: Option<serde_json::Value>,
    parent_knowledge_id: Option<i32>
) -> Result<KnowledgeObject, MCPError>

get_task_knowledge(
    task_code: String,
    requesting_agent_name: String,   // kebab-case
    knowledge_types: Option<Vec<KnowledgeType>>,
    since: Option<DateTime<Utc>>,
    tags: Option<Vec<String>>
) -> Result<Vec<KnowledgeObject>, MCPError>

search_knowledge(
    query: String,
    requesting_agent_name: String,   // kebab-case
    task_codes: Option<Vec<String>>, // task codes instead of IDs
    knowledge_types: Option<Vec<KnowledgeType>>,
    limit: Option<i32>
) -> Result<Vec<KnowledgeObject>, MCPError>

// Handoff-specific operations
initiate_handoff(
    task_code: String,
    from_agent_name: String,         // kebab-case
    to_capability: String,
    summary: String,
    confidence_score: f64,
    artifacts: serde_json::Value,
    known_limitations: Vec<String>,
    next_steps_suggestion: String
) -> Result<HandoffPackage, MCPError>

complete_handoff(
    handoff_id: i32,
    accepting_agent_name: String,    // kebab-case
    acceptance_notes: String
) -> Result<(), MCPError>
```

### Task Messages System

```rust
// Add message/comment to task
add_task_message(
    task_code: String,
    author_agent_name: String,       // kebab-case
    message_type: MessageType,
    content: String,
    reply_to_message_id: Option<i32>
) -> Result<TaskMessage, MCPError>

// Get task messages with filtering
get_task_messages(
    task_code: String,
    requesting_agent_name: String,   // kebab-case
    message_types: Vec<MessageType>, // Empty = all types, non-empty = filter by types
    since: Option<DateTime<Utc>>,
    limit: Option<i32>
) -> Result<Vec<TaskMessage>, MCPError>

// Search messages across tasks
search_task_messages(
    query: String,
    requesting_agent_name: String,   // kebab-case
    task_codes: Option<Vec<String>>, // task codes instead of IDs
    message_types: Vec<MessageType>, // Empty = all, non-empty = filter by types
    limit: Option<i32>
) -> Result<Vec<TaskMessage>, MCPError>
```

### Agent Management

```rust
// Agent registry management (one-time setup by project manager)
register_agent(
    name: String,                    // kebab-case (e.g., "frontend-developer")
    description: String,             // Up to 4000 chars describing work scope
    capabilities: Vec<String>,
    max_concurrent_tasks: i32,
    registered_by: String            // Who is registering this agent
) -> Result<AgentProfile, MCPError>

list_registered_agents() -> Result<Vec<AgentProfile>, MCPError>

// Agent lifecycle
update_agent_status(agent_name: String, status: AgentStatus) -> Result<(), MCPError>  // kebab-case
agent_heartbeat(agent_name: String, current_load: i32) -> Result<(), MCPError>       // kebab-case

// Agent discovery and matching
find_agents(
    required_capabilities: Vec<String>,
    max_load_threshold: i32,
    exclude_agents: Option<Vec<i32>>
) -> Result<Vec<AgentProfile>, MCPError>

get_agent_workload(agent_name: String) -> Result<AgentWorkloadSummary, MCPError>  // kebab-case

// Peer assistance
request_help(
    requesting_agent_name: String,  // kebab-case
    task_id: i32,
    help_type: HelpType,
    description: String,
    urgency: UrgencyLevel
) -> Result<HelpRequest, MCPError>

enum HelpType {
    TechnicalQuestion,    // Need expertise
    Blocker,             // Stuck on something
    Review,              // Need code/work review
    Clarification,       // Requirements unclear
    Escalation,          // Need higher authority
}
```

### System State and Analytics

```rust
// Project visibility
get_project_overview(
    requesting_agent_name: String,  // kebab-case
    include_completed: bool
) -> Result<ProjectOverview, MCPError>

get_dependency_graph(
    task_codes: Option<Vec<String>>, // task codes instead of IDs
    max_depth: Option<i32>
) -> Result<TaskDependencyGraph, MCPError>

// Performance analytics
get_agent_performance(
    agent_name: String,  // kebab-case
    time_range: TimeRange
) -> Result<AgentPerformanceReport, MCPError>

get_task_metrics(
    task_code: String
) -> Result<TaskMetrics, MCPError>

// System health
get_system_status() -> Result<SystemStatus, MCPError>
get_blocked_tasks_report() -> Result<Vec<BlockedTaskSummary>, MCPError>
```

## Agent Experience Design

### The Agent Workflow Loop

```
1. REGISTER → Agent announces capabilities and availability
2. DISCOVER → Agent calls discover_work() with 120s timeout (long-polling)
3. WAIT/POLL → MCP checks DB every 3s for up to 110s for available tasks
4. CLAIM → Agent atomically claims highest priority available task
5. CONTEXT → Agent downloads knowledge objects and handoff packages
6. START_WORK → Agent calls start_working_on_task() to begin time tracking
7. EXECUTE → Agent performs work while sending periodic heartbeats
8. DOCUMENT → Agent creates knowledge objects as it works
9. FINISH_WORK → Agent calls finish_working_on_task() to complete time tracking
10. HANDOFF → Agent packages results and initiates handoff to next capability
11. RELEASE → Agent releases task and returns to DISCOVER state
```

### Long-Polling Work Discovery

**Initial Project Setup**: Architects/analysts/project managers create the complete task hierarchy upfront using `create_task_hierarchy()` with proper sequencing and dependencies.

**Agent Work Discovery**: When agents call `discover_work()`:

1. **Immediate Check**: MCP first checks if tasks are available now
2. **Long-Polling**: If no tasks available, MCP waits and polls DB every 3 seconds
3. **Timeout Handling**: After 110 seconds, returns `NoTasksAvailable` 
4. **Early Return**: If tasks become available during polling, returns immediately

**Implementation Logic**:
```rust
// Pseudo-code for discover_work() implementation
let start_time = now();
let timeout = min(timeout_seconds.unwrap_or(120), 120);

loop {
    // Check for available tasks
    if let Some(tasks) = find_available_tasks_for_agent(agent_name) {
        return Ok(TasksAvailable(tasks));
    }
    
    // Check for prerequisite actions
    if let Some(action) = check_prerequisite_actions(agent_name) {
        return Ok(PrerequisiteActionRequired(action));
    }
    
    // Check timeout (leave 10s buffer for response)
    if elapsed_time(start_time) > (timeout - 10) {
        return Ok(NoTasksAvailable);
    }
    
    // Wait 3 seconds before next poll
    sleep(3_seconds);
}
```

**Benefits**:
- Agents don't waste CPU with frequent polling
- Near real-time response when new tasks become available
- Graceful timeout handling prevents infinite waiting
- Prerequisite actions can interrupt the polling loop

### Intelligent Work Discovery

Agents call `discover_work()` which returns tasks ordered by:
1. **Dependency Resolution** (only tasks with satisfied dependencies are available)
2. **Architect Sequence** (tasks created earlier in the subtask array get priority)
3. **Priority Score** (set by orchestrator or project manager agents)
4. **Capability Match Quality** (exact vs. partial capability overlap)
5. **Agent Specialization** (agents get preference for their deep expertise areas)
6. **Failure Count** (lower failure count = higher priority)

**Note**: The system fundamentally respects the sequential order from the original subtask array position combined with dependency constraints. This ensures work flows in the architect's intended logical sequence while preventing impossible assignments (blocked dependencies).

### Handoff Package Structure

When advancing a workflow or completing a task, agents create structured handoff packages:

```json
{
  "task_id": "ARCH-01",
  "from_agent": "architect-01", 
  "target_capability": "rust_developer",
  "summary": "Core authentication service architecture complete. Database schemas defined, API endpoints specified. Ready for implementation.",
  "confidence_score": 0.92,
  "artifacts": {
    "code_scaffold": "https://artifacts.com/arch-01/code.zip",
    "api_spec": "https://artifacts.com/arch-01/openapi.yaml",
    "database_schema": "https://artifacts.com/arch-01/schema.sql"
  },
  "known_limitations": [
    "Password hashing algorithm is placeholder",
    "Rate limiting not yet designed",
    "Error codes need standardization"
  ],
  "next_steps_suggestion": "Start with UserRepository implementation, focus on the registration flow first as it's the critical path.",
  "blockers_resolved": [
    "Database choice finalized as PostgreSQL", 
    "Authentication strategy settled on JWT + refresh tokens"
  ],
  "estimated_effort": 240  // minutes
}
```

## Resilience and Failure Handling

### Circuit Breaker Pattern
- Tasks track `failure_count` - number of times agents have failed on them
- After 3 failures, task automatically moves to `Quarantined` state
- Quarantined tasks require human review before re-entering workflow

### Agent Heartbeat System
- Agents send heartbeats every 60 seconds while `Active`
- Missing 3 consecutive heartbeats triggers automatic task release
- Agent status changes to `Unresponsive` until it reconnects

### Knowledge Fidelity Protection
- Handoffs require minimum `confidence_score` (configurable, default 0.7)
- Low confidence handoffs create `Review` tasks for senior agents
- Original task requirements preserved in knowledge objects to prevent drift

### Escalation Pathways
1. **Agent Self-Reports Block** → Status: `Blocked`, creates help request
2. **Task Stalls** → System detects no progress for 2+ hours, flags for review  
3. **Repeated Failures** → Circuit breaker triggers quarantine
4. **Human Intervention** → Special "admin" agent type can override any state

## Implementation Priorities

### Phase 1: Enhanced Core (Build on v1)
- [ ] Add knowledge objects table and basic CRUD operations
- [ ] Implement agent registry and heartbeat system
- [ ] Add task priority scoring and capability matching
- [ ] Create `discover_work()` and `claim_task()` functions

### Phase 2: Workflow Engine
- [ ] Add workflow definitions and step tracking
- [ ] Implement `advance_task_workflow()` with handoff packages
- [ ] Create task decomposition capabilities
- [ ] Add basic event logging

### Phase 3: Intelligence Layer
- [ ] Smart priority calculation based on dependencies and deadlines
- [ ] Agent performance tracking and reputation scoring
- [ ] Knowledge search and recommendation system
- [ ] Circuit breaker and quarantine mechanisms

### Phase 4: Advanced Features
- [ ] Real-time dashboard for project visibility
- [ ] ML-based task assignment optimization
- [ ] Integration with external tools (GitHub, Slack, etc.)
- [ ] Advanced analytics and reporting

## Success Metrics

### Technical Metrics
- **Task Throughput**: Tasks completed per day
- **Handoff Quality**: Average confidence score of handoffs
- **Agent Utilization**: Percentage of time agents spend on productive work vs. waiting
- **Knowledge Reuse**: Frequency of knowledge object references across tasks
- **Time Estimation Accuracy**: Actual vs. estimated work time per task type and agent
- **Sequence Adherence**: How often tasks complete in architect-specified order

### Operational Metrics  
- **Time to Claim**: How quickly appropriate agents claim available work
- **Context Transfer Time**: Time from handoff initiation to acceptance
- **Escalation Rate**: Percentage of tasks requiring human intervention
- **Agent Satisfaction**: Survey-based metric on ease of use and effectiveness

### Business Metrics
- **Project Velocity**: Rate of feature delivery
- **Quality Gates**: Percentage of deliverables passing review on first attempt  
- **Coordination Overhead**: Time spent on coordination vs. execution
- **Knowledge Persistence**: How well institutional knowledge is captured and reused

## Conclusion

MCP v2 transforms task management from a simple CRUD system into an intelligent orchestration platform for multi-agent collaboration. By combining autonomous agent behavior with intelligent system coordination, it enables teams of AI agents to work together as effectively as experienced human teams.

The system balances agent autonomy with global optimization, provides robust failure handling, and maintains the institutional knowledge that makes long-term collaboration possible. This creates a foundation for building truly collaborative AI systems that can tackle complex, multi-phase projects requiring diverse expertise.