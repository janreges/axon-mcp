# CORE10: Extend TaskRepository Trait - Help Requests

## Objective
Extend the TaskRepository trait to include methods for help request management, enabling agents to request and provide assistance to each other.

## Current State
The TaskRepository trait needs help request methods to support agent collaboration and escalation mechanisms.

## Required Changes

### 1. Extend TaskRepository Trait
Add to `core/src/repository.rs`:

```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Help Request Methods =====
    
    /// Create a help request
    async fn create_help_request(&self, request: NewHelpRequest) -> Result<HelpRequest>;
    
    /// Get help request by ID
    async fn get_help_request(&self, request_id: i32) -> Result<Option<HelpRequest>>;
    
    /// List help requests with filtering
    async fn list_help_requests(&self, filter: HelpRequestFilter) -> Result<Vec<HelpRequest>>;
    
    /// Resolve a help request
    async fn resolve_help_request(&self, resolution: HelpRequestResolution) -> Result<()>;
    
    /// Claim a help request
    async fn claim_help_request(&self, request_id: i32, agent_name: &str) -> Result<()>;
    
    /// Escalate a help request
    async fn escalate_help_request(&self, request_id: i32, new_urgency: Urgency) -> Result<()>;
    
    /// Get help requests for an agent
    async fn get_agent_help_requests(&self, agent_name: &str, include_resolved: bool) -> Result<Vec<HelpRequest>>;
    
    /// Get help requests by capability
    async fn get_help_by_capability(&self, capability: &str) -> Result<Vec<HelpRequest>>;
}
```

### 2. Add Help Request Types
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelpType {
    TechnicalQuestion,
    Blocker,
    Review,
    Clarification,
    Escalation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Low,
    Medium,
    High,
    Critical,
}

impl Urgency {
    /// Get numeric priority (higher = more urgent)
    pub fn priority(&self) -> i32 {
        match self {
            Urgency::Low => 1,
            Urgency::Medium => 2,
            Urgency::High => 3,
            Urgency::Critical => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpRequest {
    pub id: i32,
    pub requesting_agent_name: String,
    pub task_code: String,
    pub help_type: HelpType,
    pub description: String,
    pub urgency: Urgency,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
    pub resolution: Option<String>,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub related_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewHelpRequest {
    pub requesting_agent_name: String,
    pub task_code: String,
    pub help_type: HelpType,
    pub description: String,
    pub urgency: Urgency,
    pub related_capabilities: Vec<String>,
}
```

### 3. Add Validation Methods
```rust
impl NewHelpRequest {
    pub fn validate(&self) -> Result<()> {
        // Validate agent name
        if !self.requesting_agent_name.chars().all(|c| c.is_lowercase() || c == '-' || c.is_numeric()) {
            return Err(TaskError::Validation(
                "Agent name must be in kebab-case format".to_string()
            ));
        }
        
        // Validate description
        if self.description.is_empty() || self.description.len() > 2000 {
            return Err(TaskError::Validation(
                "Description must be between 1 and 2000 characters".to_string()
            ));
        }
        
        // Validate task code
        if self.task_code.is_empty() {
            return Err(TaskError::Validation(
                "Task code cannot be empty".to_string()
            ));
        }
        
        Ok(())
    }
}

impl HelpRequest {
    /// Check if request is open (not resolved)
    pub fn is_open(&self) -> bool {
        self.resolved_at.is_none()
    }
    
    /// Check if request is claimed
    pub fn is_claimed(&self) -> bool {
        self.claimed_by.is_some()
    }
    
    /// Calculate age in minutes
    pub fn age_minutes(&self) -> i64 {
        let now = Utc::now();
        (now - self.created_at).num_minutes()
    }
    
    /// Calculate resolution time if resolved
    pub fn resolution_time_minutes(&self) -> Option<i64> {
        self.resolved_at.map(|resolved| {
            (resolved - self.created_at).num_minutes()
        })
    }
}
```

### 4. Add Filter and Resolution Types
```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HelpRequestFilter {
    pub help_types: Vec<HelpType>,
    pub urgency_min: Option<Urgency>,
    pub status: Option<HelpRequestStatus>,
    pub task_code: Option<String>,
    pub requesting_agent: Option<String>,
    pub claimed_by: Option<String>,
    pub capabilities: Vec<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelpRequestStatus {
    Open,
    Claimed,
    Resolved,
}

impl HelpRequestFilter {
    /// Create filter for open requests
    pub fn open_only() -> Self {
        Self {
            status: Some(HelpRequestStatus::Open),
            ..Default::default()
        }
    }
    
    /// Create filter for urgent requests
    pub fn urgent_only() -> Self {
        Self {
            urgency_min: Some(Urgency::High),
            status: Some(HelpRequestStatus::Open),
            ..Default::default()
        }
    }
    
    /// Filter by capability
    pub fn by_capability(capability: &str) -> Self {
        Self {
            capabilities: vec![capability.to_string()],
            status: Some(HelpRequestStatus::Open),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpRequestResolution {
    pub request_id: i32,
    pub resolved_by: String,
    pub resolution: String,
}

impl HelpRequestResolution {
    pub fn validate(&self) -> Result<()> {
        if self.resolution.is_empty() || self.resolution.len() > 2000 {
            return Err(TaskError::Validation(
                "Resolution must be between 1 and 2000 characters".to_string()
            ));
        }
        
        Ok(())
    }
}
```

### 5. Add Help Request Notification Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpRequestNotification {
    pub request_id: i32,
    pub notification_type: HelpNotificationType,
    pub target_agents: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelpNotificationType {
    NewRequest,
    Claimed,
    Resolved,
    Escalated,
}
```

### 6. Protocol Handler Extension
```rust
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    // ... existing methods ...
    
    // Help Requests
    async fn create_help_request(&self, params: CreateHelpRequestParams) -> Result<HelpRequest>;
    async fn list_help_requests(&self, params: ListHelpRequestsParams) -> Result<Vec<HelpRequest>>;
    async fn claim_help_request(&self, params: ClaimHelpRequestParams) -> Result<()>;
    async fn resolve_help_request(&self, params: ResolveHelpRequestParams) -> Result<()>;
    async fn escalate_help_request(&self, params: EscalateHelpRequestParams) -> Result<()>;
}

// Parameter types
#[derive(Debug, Deserialize)]
pub struct CreateHelpRequestParams {
    pub requesting_agent_name: String,
    pub task_code: String,
    pub help_type: HelpType,
    pub description: String,
    pub urgency: Urgency,
    pub related_capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListHelpRequestsParams {
    pub requesting_agent: String,
    pub filter: HelpRequestFilter,
}

#[derive(Debug, Deserialize)]
pub struct ClaimHelpRequestParams {
    pub request_id: i32,
    pub agent_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolveHelpRequestParams {
    pub request_id: i32,
    pub resolved_by: String,
    pub resolution: String,
}

#[derive(Debug, Deserialize)]
pub struct EscalateHelpRequestParams {
    pub request_id: i32,
    pub escalating_agent: String,
    pub new_urgency: Urgency,
    pub reason: String,
}
```

### 7. Add Helper Methods
```rust
impl HelpType {
    /// Get suggested response time in minutes
    pub fn suggested_response_minutes(&self, urgency: Urgency) -> i32 {
        match (self, urgency) {
            (_, Urgency::Critical) => 15,
            (HelpType::Blocker, Urgency::High) => 30,
            (_, Urgency::High) => 60,
            (HelpType::Blocker, _) => 120,
            _ => 240,
        }
    }
    
    /// Check if this type can be auto-escalated
    pub fn can_auto_escalate(&self) -> bool {
        matches!(self, HelpType::Blocker | HelpType::Escalation)
    }
}

impl fmt::Display for HelpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HelpType::TechnicalQuestion => write!(f, "technical_question"),
            HelpType::Blocker => write!(f, "blocker"),
            HelpType::Review => write!(f, "review"),
            HelpType::Clarification => write!(f, "clarification"),
            HelpType::Escalation => write!(f, "escalation"),
        }
    }
}

impl fmt::Display for Urgency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Urgency::Low => write!(f, "low"),
            Urgency::Medium => write!(f, "medium"),
            Urgency::High => write!(f, "high"),
            Urgency::Critical => write!(f, "critical"),
        }
    }
}
```

## Files to Modify
- `core/src/repository.rs` - Add help request methods to trait
- `core/src/protocol.rs` - Add protocol handler methods
- `core/src/models.rs` - Add help request types or create new module
- `core/src/models/help_requests.rs` - New file for help request types

## Testing Requirements
1. Mock implementations for all new methods
2. Tests for validation methods
3. Tests for urgency priority calculations
4. Tests for response time suggestions
5. Tests for filter combinations
6. Integration tests in database crate

## Notes
- Help requests can be claimed before resolution to prevent duplicate work
- Urgency can be escalated if not resolved in time
- Related capabilities help in routing requests to suitable agents
- Consider implementing auto-escalation based on age
- Resolution should include enough detail to help with similar future issues