# CORE03: Define Agent Management Types

## Objective
Create comprehensive agent management types that enable the MCP v2 system to track and coordinate multiple AI agents with different capabilities and workloads.

## Implementation Details

### 1. Create AgentStatus Enum
Create in `core/src/models.rs` or new file `core/src/models/agents.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,          // Available for work
    Active,        // Currently working
    Blocked,       // Stuck on current task
    Unresponsive,  // Missed heartbeats
    Offline,       // Deliberately offline
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentStatus::Idle => write!(f, "idle"),
            AgentStatus::Active => write!(f, "active"),
            AgentStatus::Blocked => write!(f, "blocked"),
            AgentStatus::Unresponsive => write!(f, "unresponsive"),
            AgentStatus::Offline => write!(f, "offline"),
        }
    }
}
```

### 2. Create AgentProfile Struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: i32,
    pub name: String,                    // kebab-case format (e.g., "ui-ux-researcher")
    pub description: String,             // Up to 4000 chars describing agent's work scope
    pub capabilities: Vec<String>,       // e.g., ["rust", "architecture", "testing"]
    pub max_concurrent_tasks: i32,
    pub current_load: i32,               // Number of active tasks
    pub status: AgentStatus,
    pub preferences: serde_json::Value,  // Working hours, preferences, etc.
    pub last_heartbeat: DateTime<Utc>,
    pub reputation_score: f64,           // Based on task completion quality
    pub specializations: Vec<String>,    // Deep expertise areas
    pub registered_at: DateTime<Utc>,
    pub registered_by: String,           // Who registered this agent
}

impl AgentProfile {
    /// Check if agent can take on more work
    pub fn has_capacity(&self) -> bool {
        self.current_load < self.max_concurrent_tasks && 
        matches!(self.status, AgentStatus::Idle | AgentStatus::Active)
    }
    
    /// Check if agent has required capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
    
    /// Check if agent has all required capabilities
    pub fn has_all_capabilities(&self, required: &[String]) -> bool {
        required.iter().all(|cap| self.has_capability(cap))
    }
    
    /// Calculate capability match score (0.0 to 1.0)
    pub fn capability_match_score(&self, required: &[String]) -> f64 {
        if required.is_empty() {
            return 1.0;
        }
        
        let matched = required.iter()
            .filter(|cap| self.has_capability(cap))
            .count();
            
        matched as f64 / required.len() as f64
    }
    
    /// Check if agent is available for work
    pub fn is_available(&self) -> bool {
        matches!(self.status, AgentStatus::Idle | AgentStatus::Active) &&
        self.has_capacity()
    }
}
```

### 3. Create NewAgent Struct for Registration
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAgent {
    pub name: String,                    // Must be kebab-case
    pub description: String,             // Up to 4000 chars
    pub capabilities: Vec<String>,
    pub max_concurrent_tasks: i32,
    pub registered_by: String,
    pub specializations: Vec<String>,
    pub preferences: Option<serde_json::Value>,
}

impl NewAgent {
    pub fn validate(&self) -> Result<(), TaskError> {
        // Validate kebab-case name
        if !self.name.chars().all(|c| c.is_lowercase() || c == '-' || c.is_numeric()) {
            return Err(TaskError::Validation(
                "Agent name must be kebab-case (lowercase with hyphens)".to_string()
            ));
        }
        
        // Validate description length
        if self.description.len() > 4000 {
            return Err(TaskError::Validation(
                "Agent description must be 4000 characters or less".to_string()
            ));
        }
        
        // Validate max concurrent tasks
        if self.max_concurrent_tasks < 1 || self.max_concurrent_tasks > 100 {
            return Err(TaskError::Validation(
                "Max concurrent tasks must be between 1 and 100".to_string()
            ));
        }
        
        // Validate capabilities not empty
        if self.capabilities.is_empty() {
            return Err(TaskError::Validation(
                "Agent must have at least one capability".to_string()
            ));
        }
        
        Ok(())
    }
}
```

### 4. Create AgentWorkloadSummary Struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorkloadSummary {
    pub agent_name: String,
    pub status: AgentStatus,
    pub active_tasks: Vec<TaskSummary>,
    pub completed_today: i32,
    pub average_task_duration_minutes: f64,
    pub current_load_percentage: f64,  // current_load / max_concurrent_tasks * 100
    pub last_task_completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub code: String,
    pub name: String,
    pub state: TaskState,
    pub started_at: DateTime<Utc>,
    pub estimated_effort: Option<i32>,
}
```

### 5. Create Helper Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilityMatch {
    pub agent_name: String,
    pub match_score: f64,          // 0.0 to 1.0
    pub matched_capabilities: Vec<String>,
    pub missing_capabilities: Vec<String>,
    pub is_specialized: bool,      // Has specialization in required area
    pub current_load: i32,
    pub reputation_score: f64,
}

impl AgentCapabilityMatch {
    /// Calculate overall suitability score combining match, load, and reputation
    pub fn suitability_score(&self) -> f64 {
        let load_factor = 1.0 - (self.current_load as f64 / 10.0).min(1.0);
        let specialization_bonus = if self.is_specialized { 0.2 } else { 0.0 };
        
        (self.match_score * 0.4) + 
        (self.reputation_score * 0.3) + 
        (load_factor * 0.3) + 
        specialization_bonus
    }
}
```

### 6. Add Validation Constants
```rust
pub mod agent_validation {
    pub const MAX_AGENT_NAME_LENGTH: usize = 50;
    pub const MAX_DESCRIPTION_LENGTH: usize = 4000;
    pub const MAX_CONCURRENT_TASKS: i32 = 100;
    pub const HEARTBEAT_INTERVAL_SECONDS: i64 = 60;
    pub const HEARTBEAT_TIMEOUT_COUNT: i32 = 3;
    
    /// Special agent name that means "all agents"
    pub const ALL_AGENTS_KEYWORD: &str = "all";
}
```

## Files to Create/Modify
- `core/src/models.rs` - Add agent types or create separate module
- `core/src/models/agents.rs` - New file if separating agent types
- `core/src/lib.rs` - Export new types

## Testing Requirements
1. Unit tests for all validation methods
2. Tests for capability matching logic
3. Tests for suitability score calculations
4. Serialization/deserialization tests
5. Edge cases (empty capabilities, invalid names, etc.)

## Database Considerations
These types will map to the `agents` table with JSON columns for:
- capabilities (JSON array)
- preferences (JSON object)
- specializations (JSON array)