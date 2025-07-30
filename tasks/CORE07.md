# CORE07: Extend TaskRepository Trait - Agents

## Objective
Extend the TaskRepository trait to include methods for agent management, enabling registration, tracking, and coordination of multiple AI agents.

## Current State
The TaskRepository trait needs agent management methods to support the MCP v2 multi-agent system.

## Required Changes

### 1. Extend TaskRepository Trait
Add to `core/src/repository.rs`:

```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Agent Management Methods =====
    
    /// Register a new agent in the system
    async fn register_agent(&self, agent: NewAgent) -> Result<AgentProfile>;
    
    /// Get agent by name
    async fn get_agent(&self, agent_name: &str) -> Result<Option<AgentProfile>>;
    
    /// Get all agents with optional filtering
    async fn list_agents(&self, filter: AgentFilter) -> Result<Vec<AgentProfile>>;
    
    /// Update agent status
    async fn update_agent_status(&self, agent_name: &str, status: AgentStatus) -> Result<()>;
    
    /// Update agent heartbeat
    async fn heartbeat(&self, agent_name: &str) -> Result<()>;
    
    /// Update agent workload
    async fn update_agent_load(&self, agent_name: &str, current_load: i32) -> Result<()>;
    
    /// Find best agents for a capability
    async fn find_agents_by_capability(&self, capability: &str, limit: i32) -> Result<Vec<AgentCapabilityMatch>>;
    
    /// Get agent workload summary
    async fn get_agent_workload(&self, agent_name: &str) -> Result<AgentWorkloadSummary>;
    
    /// Update agent reputation
    async fn update_agent_reputation(&self, agent_name: &str, delta: f64) -> Result<()>;
    
    /// Deactivate agent (set offline)
    async fn deactivate_agent(&self, agent_name: &str) -> Result<()>;
    
    /// Get agents needing heartbeat check
    async fn get_unresponsive_agents(&self, timeout_seconds: i64) -> Result<Vec<AgentProfile>>;
}
```

### 2. Add Agent Filter Types
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentFilter {
    pub status: Option<AgentStatus>,
    pub has_capability: Option<String>,
    pub has_capacity: bool,
    pub min_reputation: Option<f64>,
    pub specialization: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl AgentFilter {
    /// Filter for available agents only
    pub fn available_only() -> Self {
        Self {
            has_capacity: true,
            status: Some(AgentStatus::Idle),
            ..Default::default()
        }
    }
    
    /// Filter by capability
    pub fn with_capability(capability: &str) -> Self {
        Self {
            has_capability: Some(capability.to_string()),
            ..Default::default()
        }
    }
    
    /// Filter by minimum reputation
    pub fn with_min_reputation(score: f64) -> Self {
        Self {
            min_reputation: Some(score),
            ..Default::default()
        }
    }
}
```

### 3. Add Agent Statistics Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatistics {
    pub agent_name: String,
    pub total_tasks_completed: i32,
    pub average_task_duration_minutes: f64,
    pub success_rate: f64,
    pub current_streak: i32,  // Consecutive successful tasks
    pub specialization_scores: HashMap<String, f64>,
    pub last_7_days_tasks: i32,
    pub last_30_days_tasks: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamWorkloadSummary {
    pub total_agents: i32,
    pub active_agents: i32,
    pub total_capacity: i32,
    pub current_total_load: i32,
    pub agents_at_capacity: Vec<String>,
    pub idle_agents: Vec<String>,
    pub blocked_agents: Vec<String>,
    pub average_load_percentage: f64,
}
```

### 4. Add Work Discovery Methods
```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Work Discovery Methods =====
    
    /// Discover work for an agent with specific capabilities
    async fn discover_work(&self, params: WorkDiscoveryParams) -> Result<Vec<Task>>;
    
    /// Get team workload summary
    async fn get_team_workload(&self) -> Result<TeamWorkloadSummary>;
    
    /// Get agent statistics
    async fn get_agent_statistics(&self, agent_name: &str) -> Result<AgentStatistics>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkDiscoveryParams {
    pub agent_name: String,
    pub capabilities: Vec<String>,
    pub max_tasks: i32,
    pub include_types: Vec<TaskState>,
    pub exclude_codes: Vec<String>,
    pub min_priority: Option<i32>,
}

impl WorkDiscoveryParams {
    /// Create params for standard work discovery
    pub fn standard(agent_name: &str, capabilities: Vec<String>) -> Self {
        Self {
            agent_name: agent_name.to_string(),
            capabilities,
            max_tasks: 10,
            include_types: vec![
                TaskState::Created,
                TaskState::InProgress,
                TaskState::Review,
                TaskState::PendingHandoff,
            ],
            exclude_codes: vec![],
            min_priority: None,
        }
    }
}
```

### 5. Protocol Handler Extension
Add to `ProtocolHandler` trait:

```rust
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    // ... existing methods ...
    
    // Agent Management
    async fn register_agent(&self, params: RegisterAgentParams) -> Result<AgentProfile>;
    async fn get_agent(&self, params: GetAgentParams) -> Result<Option<AgentProfile>>;
    async fn list_agents(&self, params: ListAgentsParams) -> Result<Vec<AgentProfile>>;
    async fn heartbeat(&self, params: HeartbeatParams) -> Result<()>;
    async fn discover_work(&self, params: DiscoverWorkParams) -> Result<Vec<Task>>;
    async fn get_team_status(&self, params: GetTeamStatusParams) -> Result<TeamWorkloadSummary>;
}

// Parameter types
#[derive(Debug, Deserialize)]
pub struct RegisterAgentParams {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub max_concurrent_tasks: i32,
    pub registered_by: String,
    pub specializations: Vec<String>,
    pub preferences: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct GetAgentParams {
    pub agent_name: String,
    pub requesting_agent: String,
}

#[derive(Debug, Deserialize)]
pub struct ListAgentsParams {
    pub requesting_agent: String,
    pub status: Option<AgentStatus>,
    pub capability: Option<String>,
    pub available_only: bool,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatParams {
    pub agent_name: String,
    pub current_load: i32,
    pub status: AgentStatus,
}

#[derive(Debug, Deserialize)]
pub struct DiscoverWorkParams {
    pub agent_name: String,
    pub max_tasks: i32,
}

#[derive(Debug, Deserialize)]
pub struct GetTeamStatusParams {
    pub requesting_agent: String,
}
```

### 6. Add Helper Methods
```rust
impl AgentProfile {
    /// Calculate time since last heartbeat
    pub fn seconds_since_heartbeat(&self) -> i64 {
        let now = Utc::now();
        (now - self.last_heartbeat).num_seconds()
    }
    
    /// Check if agent should be marked unresponsive
    pub fn should_mark_unresponsive(&self, timeout_seconds: i64) -> bool {
        self.seconds_since_heartbeat() > timeout_seconds &&
        matches!(self.status, AgentStatus::Active | AgentStatus::Idle)
    }
    
    /// Get load percentage
    pub fn load_percentage(&self) -> f64 {
        if self.max_concurrent_tasks == 0 {
            return 0.0;
        }
        (self.current_load as f64 / self.max_concurrent_tasks as f64) * 100.0
    }
}
```

## Files to Modify
- `core/src/repository.rs` - Add agent methods to trait
- `core/src/protocol.rs` - Add protocol handler methods
- `core/src/models.rs` - Ensure agent types are imported
- `core/src/models/agents.rs` - Add new helper methods

## Testing Requirements
1. Mock implementations for all new methods
2. Tests for agent filtering
3. Tests for work discovery logic
4. Tests for heartbeat timeout detection
5. Tests for capability matching
6. Integration tests in database crate

## Notes
- Agent names are unique and in kebab-case
- Heartbeat updates both timestamp and optionally status/load
- Work discovery respects agent capabilities and current load
- Reputation updates should be atomic (prevent race conditions)
- Unresponsive agents should be detected and marked automatically