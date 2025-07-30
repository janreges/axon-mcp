# CORE09: Extend TaskRepository Trait - Analytics

## Objective
Extend the TaskRepository trait to include methods for system analytics, metrics, and reporting to support monitoring and optimization of the multi-agent system.

## Current State
The TaskRepository trait needs analytics methods to provide insights into system performance and agent efficiency.

## Required Changes

### 1. Extend TaskRepository Trait - System Events
Add to `core/src/repository.rs`:

```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== System Events and Audit Trail =====
    
    /// Log a system event
    async fn log_event(&self, event: SystemEvent) -> Result<()>;
    
    /// Query system events
    async fn get_events(&self, filter: EventFilter) -> Result<Vec<SystemEvent>>;
    
    /// Get events for a specific task
    async fn get_task_events(&self, task_code: &str, limit: i32) -> Result<Vec<SystemEvent>>;
    
    /// Get events by correlation ID
    async fn get_correlated_events(&self, correlation_id: &str) -> Result<Vec<SystemEvent>>;
    
    /// Get event statistics
    async fn get_event_stats(&self, time_range: TimeRange) -> Result<EventStatistics>;
}
```

### 2. Add System Event Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub id: i32,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub actor_type: ActorType,
    pub actor_id: String,
    pub task_code: Option<String>,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    Agent,
    System,
    Human,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    pub event_types: Vec<String>,
    pub actor_type: Option<ActorType>,
    pub actor_id: Option<String>,
    pub task_code: Option<String>,
    pub time_range: TimeRange,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Create a time range for the last N hours
    pub fn last_hours(hours: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::hours(hours);
        Self { start, end }
    }
    
    /// Create a time range for today
    pub fn today() -> Self {
        let now = Utc::now();
        let start = now.date_naive().and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        Self { start, end: now }
    }
}
```

### 3. Add Analytics Methods
```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Analytics and Metrics =====
    
    /// Get task completion metrics
    async fn get_task_metrics(&self, time_range: TimeRange) -> Result<TaskMetrics>;
    
    /// Get agent performance metrics
    async fn get_agent_metrics(&self, agent_name: &str, time_range: TimeRange) -> Result<AgentMetrics>;
    
    /// Get system-wide metrics
    async fn get_system_metrics(&self) -> Result<SystemMetrics>;
    
    /// Get task duration statistics by state
    async fn get_duration_stats(&self, time_range: TimeRange) -> Result<DurationStatistics>;
    
    /// Get help request analytics
    async fn get_help_request_stats(&self, time_range: TimeRange) -> Result<HelpRequestStatistics>;
    
    /// Get workflow completion rates
    async fn get_workflow_metrics(&self, workflow_id: Option<i32>) -> Result<WorkflowMetrics>;
}
```

### 4. Add Metrics Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub total_created: i32,
    pub total_completed: i32,
    pub total_failed: i32,
    pub completion_rate: f64,
    pub average_duration_minutes: f64,
    pub by_state: HashMap<TaskState, i32>,
    pub by_priority: HashMap<i32, i32>,
    pub overdue_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub agent_name: String,
    pub tasks_completed: i32,
    pub tasks_failed: i32,
    pub average_duration_minutes: f64,
    pub success_rate: f64,
    pub current_load: i32,
    pub total_active_minutes: i32,
    pub idle_time_percentage: f64,
    pub handoffs_created: i32,
    pub handoffs_received: i32,
    pub messages_sent: i32,
    pub knowledge_objects_created: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub total_agents: i32,
    pub active_agents: i32,
    pub total_tasks: i32,
    pub active_tasks: i32,
    pub tasks_per_hour: f64,
    pub average_queue_time_minutes: f64,
    pub system_load_percentage: f64,
    pub total_messages: i64,
    pub total_knowledge_objects: i64,
    pub database_size_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStatistics {
    pub by_state: HashMap<TaskState, DurationStats>,
    pub overall: DurationStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStats {
    pub min_minutes: f64,
    pub max_minutes: f64,
    pub avg_minutes: f64,
    pub median_minutes: f64,
    pub p95_minutes: f64,
    pub p99_minutes: f64,
    pub sample_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStatistics {
    pub total_events: i64,
    pub by_type: HashMap<String, i64>,
    pub by_actor_type: HashMap<ActorType, i64>,
    pub events_per_hour: f64,
    pub peak_hour: Option<DateTime<Utc>>,
}
```

### 5. Add Help Request Analytics
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpRequestStatistics {
    pub total_requests: i32,
    pub resolved_requests: i32,
    pub resolution_rate: f64,
    pub average_resolution_time_minutes: f64,
    pub by_type: HashMap<String, i32>,
    pub by_urgency: HashMap<String, i32>,
    pub top_requesters: Vec<(String, i32)>,
    pub top_resolvers: Vec<(String, i32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetrics {
    pub workflow_id: Option<i32>,
    pub total_executions: i32,
    pub completed_executions: i32,
    pub completion_rate: f64,
    pub average_duration_minutes: f64,
    pub step_metrics: Vec<StepMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepMetric {
    pub step_id: String,
    pub step_name: String,
    pub execution_count: i32,
    pub average_duration_minutes: f64,
    pub success_rate: f64,
    pub average_confidence: f64,
}
```

### 6. Add Work Session Methods
```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Work Sessions =====
    
    /// Start a work session
    async fn start_work_session(&self, agent_name: &str, task_code: &str) -> Result<i32>;
    
    /// End a work session
    async fn end_work_session(&self, session_id: i32, notes: Option<String>) -> Result<()>;
    
    /// Update work session (heartbeat)
    async fn update_work_session(&self, session_id: i32) -> Result<()>;
    
    /// Get active work sessions
    async fn get_active_sessions(&self) -> Result<Vec<WorkSession>>;
    
    /// Get work sessions for a task
    async fn get_task_sessions(&self, task_code: &str) -> Result<Vec<WorkSession>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSession {
    pub id: i32,
    pub agent_name: String,
    pub task_code: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub total_active_minutes: i32,
    pub interruptions: Vec<Interruption>,
    pub is_active: bool,
    pub completion_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interruption {
    pub timestamp: DateTime<Utc>,
    pub reason: String,
    pub duration_minutes: i32,
}
```

### 7. Protocol Handler Extension
```rust
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    // ... existing methods ...
    
    // Analytics
    async fn get_task_metrics(&self, params: GetMetricsParams) -> Result<TaskMetrics>;
    async fn get_agent_metrics(&self, params: GetAgentMetricsParams) -> Result<AgentMetrics>;
    async fn get_system_status(&self, params: GetSystemStatusParams) -> Result<SystemMetrics>;
    
    // Events
    async fn log_event(&self, params: LogEventParams) -> Result<()>;
    async fn get_events(&self, params: GetEventsParams) -> Result<Vec<SystemEvent>>;
    
    // Work Sessions
    async fn start_session(&self, params: StartSessionParams) -> Result<i32>;
    async fn end_session(&self, params: EndSessionParams) -> Result<()>;
}

// Parameter types
#[derive(Debug, Deserialize)]
pub struct GetMetricsParams {
    pub requesting_agent: String,
    pub hours: i64,  // Look back N hours
}

#[derive(Debug, Deserialize)]
pub struct GetAgentMetricsParams {
    pub agent_name: String,
    pub requesting_agent: String,
    pub hours: i64,
}

#[derive(Debug, Deserialize)]
pub struct LogEventParams {
    pub event_type: String,
    pub actor_type: ActorType,
    pub actor_id: String,
    pub task_code: Option<String>,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StartSessionParams {
    pub agent_name: String,
    pub task_code: String,
}
```

## Files to Modify
- `core/src/repository.rs` - Add analytics methods to trait
- `core/src/protocol.rs` - Add protocol handler methods
- `core/src/models.rs` - Add analytics types or create new module
- `core/src/models/analytics.rs` - New file for analytics types

## Testing Requirements
1. Mock implementations for all new methods
2. Tests for time range calculations
3. Tests for metrics aggregation logic
4. Tests for event filtering
5. Tests for work session tracking
6. Integration tests in database crate

## Notes
- Metrics should be calculated efficiently using SQL aggregations
- Event logging should be asynchronous to not block operations
- Work sessions track actual time spent (excluding interruptions)
- Analytics queries should use appropriate indexes for performance
- Consider caching frequently accessed metrics