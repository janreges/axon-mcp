# PROTOCOL04: Implement Agent Management Handlers

## Objective
Implement all agent management protocol handlers, enabling agent registration, capability tracking, heartbeat monitoring, and status management through the MCP protocol.

## Implementation Details

### 1. Extend Protocol Handler with Agent Methods
In `mcp-protocol/src/handler.rs`, add agent management implementations:

```rust
// Add to the existing McpProtocolHandler implementation
impl<R: TaskRepository> McpProtocolHandler<R> {
    // ... existing methods ...
    
    // ===== Agent Management Methods =====
    
    async fn handle_register_agent(&self, params: RegisterAgentParams) -> Result<AgentProfile> {
        // Validate capabilities are not empty
        if params.capabilities.is_empty() {
            return Err(TaskError::Validation(
                "Agent must have at least one capability".to_string()
            ));
        }
        
        // Validate specializations are subset of capabilities
        for spec in &params.specializations {
            if !params.capabilities.contains(spec) {
                return Err(TaskError::Validation(
                    format!("Specialization {} not in capabilities", spec)
                ));
            }
        }
        
        let agent = NewAgentProfile {
            name: params.name,
            capabilities: params.capabilities,
            specializations: params.specializations.unwrap_or_default(),
            description: params.description,
            max_concurrent_tasks: params.max_concurrent_tasks.unwrap_or(5),
            preferred_task_types: params.preferred_task_types,
            timezone: params.timezone,
        };
        
        let created = self.repository.register_agent(agent).await?;
        
        // Log registration event
        self.log_agent_event("agent_registered", &created).await?;
        
        // Set initial status to idle
        self.repository.update_agent_status(&created.name, AgentStatus::Idle).await?;
        
        Ok(created)
    }
    
    async fn handle_get_agent(&self, params: GetAgentParams) -> Result<Option<AgentProfile>> {
        self.repository.get_agent(&params.name).await
    }
    
    async fn handle_list_agents(&self, params: ListAgentsParams) -> Result<Vec<AgentProfile>> {
        let filter = AgentFilter {
            status: params.status.and_then(|s| AgentStatus::from_str(&s).ok()),
            capabilities: params.capabilities.unwrap_or_default(),
            available_only: params.available_only.unwrap_or(false),
            min_reputation: params.min_reputation,
            max_load_percentage: params.max_load_percentage,
            limit: params.limit,
            offset: params.offset,
        };
        
        self.repository.list_agents(filter).await
    }
    
    async fn handle_update_agent(&self, params: UpdateAgentParams) -> Result<AgentProfile> {
        // Get existing agent
        let mut agent = self.repository
            .get_agent(&params.name)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", params.name)))?;
        
        // Update fields if provided
        if let Some(caps) = params.capabilities {
            if caps.is_empty() {
                return Err(TaskError::Validation(
                    "Cannot remove all capabilities".to_string()
                ));
            }
            agent.capabilities = caps;
        }
        
        if let Some(specs) = params.specializations {
            // Validate specializations are subset of capabilities
            for spec in &specs {
                if !agent.capabilities.contains(spec) {
                    return Err(TaskError::Validation(
                        format!("Specialization {} not in capabilities", spec)
                    ));
                }
            }
            agent.specializations = specs;
        }
        
        if let Some(max_tasks) = params.max_concurrent_tasks {
            if max_tasks < 1 {
                return Err(TaskError::Validation(
                    "Max concurrent tasks must be at least 1".to_string()
                ));
            }
            agent.max_concurrent_tasks = max_tasks;
        }
        
        if let Some(desc) = params.description {
            agent.description = Some(desc);
        }
        
        if let Some(prefs) = params.preferred_task_types {
            agent.preferred_task_types = prefs;
        }
        
        if let Some(tz) = params.timezone {
            agent.timezone = Some(tz);
        }
        
        let updated = self.repository.update_agent(agent).await?;
        
        // Log update event
        self.log_agent_event("agent_updated", &updated).await?;
        
        Ok(updated)
    }
    
    async fn handle_heartbeat(&self, params: HeartbeatParams) -> Result<HeartbeatResponse> {
        // Update last heartbeat
        self.repository.update_agent_heartbeat(
            &params.agent_name,
            params.current_load,
            params.status.and_then(|s| AgentStatus::from_str(&s).ok()),
        ).await?;
        
        // Get pending notifications for agent
        let notifications = self.get_agent_notifications(&params.agent_name).await?;
        
        // Get recommended work
        let work_available = if params.request_work.unwrap_or(false) {
            self.repository.discover_work(WorkDiscoveryParams {
                agent_name: params.agent_name.clone(),
                capabilities: vec![], // Will be fetched from agent profile
                max_tasks: 3,
                include_types: vec![TaskState::Created, TaskState::PendingHandoff],
                exclude_codes: vec![],
                min_priority: None,
            }).await.unwrap_or_default()
        } else {
            vec![]
        };
        
        Ok(HeartbeatResponse {
            agent_name: params.agent_name,
            next_heartbeat_seconds: 30,
            notifications,
            work_available: if work_available.is_empty() { None } else { Some(work_available) },
            system_status: "operational".to_string(),
        })
    }
    
    async fn handle_update_status(&self, params: UpdateStatusParams) -> Result<()> {
        let status = AgentStatus::from_str(&params.status)
            .map_err(|_| TaskError::Validation(format!("Invalid status: {}", params.status)))?;
        
        // Validate status transition
        let current = self.repository
            .get_agent(&params.agent_name)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", params.agent_name)))?;
        
        if !self.is_valid_status_transition(&current.status, &status) {
            return Err(TaskError::Validation(
                format!("Invalid status transition from {:?} to {:?}", current.status, status)
            ));
        }
        
        self.repository.update_agent_status(&params.agent_name, status).await?;
        
        // Log status change
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "agent_status_changed".to_string(),
            actor_type: ActorType::Agent,
            actor_id: params.agent_name.clone(),
            task_code: None,
            payload: serde_json::json!({
                "old_status": current.status.to_string(),
                "new_status": status.to_string(),
                "reason": params.reason,
            }),
            correlation_id: None,
        };
        
        self.repository.log_event(event).await?;
        
        Ok(())
    }
    
    async fn handle_find_by_capability(&self, params: FindByCapabilityParams) -> Result<Vec<AgentCapabilityMatch>> {
        self.repository.find_agents_by_capability(
            &params.capability,
            params.limit.unwrap_or(10),
        ).await
    }
    
    async fn handle_get_agent_metrics(&self, params: GetAgentMetricsParams) -> Result<AgentMetrics> {
        self.repository.get_agent_metrics(
            &params.agent_name,
            params.since.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        ).await
    }
    
    // Helper methods
    
    async fn log_agent_event(&self, event_type: &str, agent: &AgentProfile) -> Result<()> {
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            actor_type: ActorType::Agent,
            actor_id: agent.name.clone(),
            task_code: None,
            payload: serde_json::json!({
                "capabilities": agent.capabilities,
                "status": agent.status.to_string(),
                "current_load": agent.current_load,
            }),
            correlation_id: Some(format!("agent-{}", agent.name)),
        };
        
        self.repository.log_event(event).await
    }
    
    fn is_valid_status_transition(&self, from: &AgentStatus, to: &AgentStatus) -> bool {
        match (from, to) {
            // Can always go offline
            (_, AgentStatus::Offline) => true,
            // Can go from offline to idle
            (AgentStatus::Offline, AgentStatus::Idle) => true,
            // Can go between idle and active
            (AgentStatus::Idle, AgentStatus::Active) => true,
            (AgentStatus::Active, AgentStatus::Idle) => true,
            // Can go to maintenance from idle or offline
            (AgentStatus::Idle, AgentStatus::Maintenance) => true,
            (AgentStatus::Offline, AgentStatus::Maintenance) => true,
            // Can go from maintenance to idle
            (AgentStatus::Maintenance, AgentStatus::Idle) => true,
            // Same status is valid (no-op)
            (a, b) if a == b => true,
            // Everything else is invalid
            _ => false,
        }
    }
    
    async fn get_agent_notifications(&self, agent_name: &str) -> Result<Vec<AgentNotification>> {
        // Get unread help requests that match agent capabilities
        let agent = self.repository.get_agent(agent_name).await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", agent_name)))?;
        
        let mut notifications = Vec::new();
        
        // Check for help requests matching capabilities
        for capability in &agent.capabilities {
            let help_requests = self.repository
                .get_help_by_capability(capability)
                .await?;
            
            for request in help_requests.into_iter().take(3) {
                notifications.push(AgentNotification {
                    notification_type: "help_request_available".to_string(),
                    title: format!("{} help needed", request.help_type),
                    message: request.description.clone(),
                    task_code: Some(request.task_code),
                    urgency: Some(request.urgency.to_string()),
                    action_required: Some("claim_help_request".to_string()),
                    metadata: serde_json::json!({
                        "help_request_id": request.id,
                        "capability": capability,
                    }),
                });
            }
        }
        
        // Check for pending handoffs
        let handoffs = self.repository
            .list_handoff_packages(HandoffFilter {
                to_agent: Some(agent_name.to_string()),
                status: Some(HandoffStatus::Pending),
                ..Default::default()
            })
            .await?;
        
        for handoff in handoffs.into_iter().take(3) {
            notifications.push(AgentNotification {
                notification_type: "handoff_pending".to_string(),
                title: format!("Task handoff from {}", handoff.from_agent_name),
                message: handoff.context.clone(),
                task_code: Some(handoff.task_code),
                urgency: Some("high".to_string()),
                action_required: Some("accept_handoff".to_string()),
                metadata: serde_json::json!({
                    "handoff_id": handoff.id,
                }),
            });
        }
        
        Ok(notifications)
    }
}
```

### 2. Add Agent-Related JSON-RPC Parameters
In `mcp-protocol/src/params.rs`, add agent parameters:

```rust
use core::models::{AgentProfile, AgentStatus, AgentCapabilityMatch};

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterAgentParams {
    pub name: String,
    pub capabilities: Vec<String>,
    pub specializations: Option<Vec<String>>,
    pub description: Option<String>,
    pub max_concurrent_tasks: Option<i32>,
    pub preferred_task_types: Option<serde_json::Value>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetAgentParams {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListAgentsParams {
    pub status: Option<String>,
    pub capabilities: Option<Vec<String>>,
    pub available_only: Option<bool>,
    pub min_reputation: Option<f64>,
    pub max_load_percentage: Option<f64>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAgentParams {
    pub name: String,
    pub capabilities: Option<Vec<String>>,
    pub specializations: Option<Vec<String>>,
    pub max_concurrent_tasks: Option<i32>,
    pub description: Option<String>,
    pub preferred_task_types: Option<serde_json::Value>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeartbeatParams {
    pub agent_name: String,
    pub current_load: i32,
    pub status: Option<String>,
    pub request_work: Option<bool>,
    pub metrics: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateStatusParams {
    pub agent_name: String,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FindByCapabilityParams {
    pub capability: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetAgentMetricsParams {
    pub agent_name: String,
    pub since: Option<String>, // ISO 8601
}

// Response types
#[derive(Debug, Clone, Serialize)]
pub struct HeartbeatResponse {
    pub agent_name: String,
    pub next_heartbeat_seconds: i32,
    pub notifications: Vec<AgentNotification>,
    pub work_available: Option<Vec<Task>>,
    pub system_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentNotification {
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub task_code: Option<String>,
    pub urgency: Option<String>,
    pub action_required: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentProfile>,
    pub total_count: i32,
    pub active_count: i32,
    pub idle_count: i32,
}
```

### 3. Create Agent Monitoring Service
In `mcp-protocol/src/services/agent_monitor.rs`:

```rust
use crate::transport::sse::SseTransport;
use core::{models::*, repository::TaskRepository};
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc, Duration};

pub struct AgentMonitoringService<R: TaskRepository> {
    repository: Arc<R>,
    agent_states: Arc<RwLock<HashMap<String, AgentState>>>,
    heartbeat_timeout: Duration,
}

#[derive(Debug, Clone)]
struct AgentState {
    last_heartbeat: DateTime<Utc>,
    status: AgentStatus,
    current_load: i32,
    consecutive_failures: i32,
}

impl<R: TaskRepository> AgentMonitoringService<R> {
    pub fn new(repository: Arc<R>, heartbeat_timeout_seconds: i64) -> Self {
        Self {
            repository,
            agent_states: Arc::new(RwLock::new(HashMap::new())),
            heartbeat_timeout: Duration::seconds(heartbeat_timeout_seconds),
        }
    }
    
    /// Record agent heartbeat
    pub async fn record_heartbeat(
        &self,
        agent_name: &str,
        current_load: i32,
        status: AgentStatus,
    ) -> Result<()> {
        let mut states = self.agent_states.write().await;
        
        states.insert(agent_name.to_string(), AgentState {
            last_heartbeat: Utc::now(),
            status,
            current_load,
            consecutive_failures: 0,
        });
        
        Ok(())
    }
    
    /// Check for agents that missed heartbeats
    pub async fn check_agent_health(&self) -> Result<Vec<String>> {
        let now = Utc::now();
        let mut states = self.agent_states.write().await;
        let mut unhealthy_agents = Vec::new();
        
        for (agent_name, state) in states.iter_mut() {
            if now - state.last_heartbeat > self.heartbeat_timeout {
                state.consecutive_failures += 1;
                
                // Mark agent as offline after 3 missed heartbeats
                if state.consecutive_failures >= 3 && state.status != AgentStatus::Offline {
                    self.repository.update_agent_status(
                        agent_name,
                        AgentStatus::Offline,
                    ).await?;
                    
                    // Log event
                    let event = SystemEvent {
                        id: 0,
                        timestamp: now,
                        event_type: "agent_offline_timeout".to_string(),
                        actor_type: ActorType::System,
                        actor_id: "monitoring-service".to_string(),
                        task_code: None,
                        payload: serde_json::json!({
                            "agent_name": agent_name,
                            "last_heartbeat": state.last_heartbeat.to_rfc3339(),
                            "consecutive_failures": state.consecutive_failures,
                        }),
                        correlation_id: Some(format!("agent-{}", agent_name)),
                    };
                    
                    self.repository.log_event(event).await?;
                    
                    state.status = AgentStatus::Offline;
                    unhealthy_agents.push(agent_name.clone());
                }
            }
        }
        
        Ok(unhealthy_agents)
    }
    
    /// Get agent load balancing recommendations
    pub async fn get_load_recommendations(&self) -> Result<LoadBalancingRecommendations> {
        let agents = self.repository.list_agents(AgentFilter::default()).await?;
        
        let mut overloaded = Vec::new();
        let mut underutilized = Vec::new();
        let mut balanced = Vec::new();
        
        for agent in agents {
            let load_percentage = if agent.max_concurrent_tasks > 0 {
                (agent.current_load as f64 / agent.max_concurrent_tasks as f64) * 100.0
            } else {
                0.0
            };
            
            if load_percentage > 80.0 {
                overloaded.push(LoadInfo {
                    agent_name: agent.name,
                    current_load: agent.current_load,
                    max_load: agent.max_concurrent_tasks,
                    load_percentage,
                });
            } else if load_percentage < 20.0 && agent.status == AgentStatus::Active {
                underutilized.push(LoadInfo {
                    agent_name: agent.name,
                    current_load: agent.current_load,
                    max_load: agent.max_concurrent_tasks,
                    load_percentage,
                });
            } else {
                balanced.push(LoadInfo {
                    agent_name: agent.name,
                    current_load: agent.current_load,
                    max_load: agent.max_concurrent_tasks,
                    load_percentage,
                });
            }
        }
        
        Ok(LoadBalancingRecommendations {
            overloaded_agents: overloaded,
            underutilized_agents: underutilized,
            balanced_agents: balanced,
            recommendations: self.generate_recommendations(&overloaded, &underutilized),
        })
    }
    
    fn generate_recommendations(
        &self,
        overloaded: &[LoadInfo],
        underutilized: &[LoadInfo],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if !overloaded.is_empty() {
            recommendations.push(format!(
                "Consider redistributing tasks from overloaded agents: {}",
                overloaded.iter()
                    .map(|l| &l.agent_name)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        
        if !underutilized.is_empty() {
            recommendations.push(format!(
                "Underutilized agents available for more work: {}",
                underutilized.iter()
                    .map(|l| &l.agent_name)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        
        if overloaded.len() > underutilized.len() * 2 {
            recommendations.push(
                "Consider scaling up by registering more agents".to_string()
            );
        }
        
        recommendations
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadBalancingRecommendations {
    pub overloaded_agents: Vec<LoadInfo>,
    pub underutilized_agents: Vec<LoadInfo>,
    pub balanced_agents: Vec<LoadInfo>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadInfo {
    pub agent_name: String,
    pub current_load: i32,
    pub max_load: i32,
    pub load_percentage: f64,
}
```

### 4. Create Agent Discovery Service
In `mcp-protocol/src/services/agent_discovery.rs`:

```rust
use core::{models::*, repository::TaskRepository};
use std::collections::HashMap;

pub struct AgentDiscoveryService<R: TaskRepository> {
    repository: Arc<R>,
}

impl<R: TaskRepository> AgentDiscoveryService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
    
    /// Find best agent for a task
    pub async fn find_best_agent_for_task(
        &self,
        task: &Task,
    ) -> Result<Option<AgentRecommendation>> {
        // Parse required capabilities
        let required_caps = task.required_capabilities
            .as_ref()
            .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
            .unwrap_or_default();
        
        if required_caps.is_empty() {
            // Any available agent can handle it
            let agents = self.repository.list_agents(
                AgentFilter::available_only()
            ).await?;
            
            if let Some(agent) = agents.into_iter()
                .min_by_key(|a| a.current_load) {
                return Ok(Some(AgentRecommendation {
                    agent_name: agent.name,
                    match_score: 1.0,
                    capability_matches: vec![],
                    load_factor: 1.0 - (agent.current_load as f64 / agent.max_concurrent_tasks as f64),
                    specialization_bonus: 0.0,
                    recommendation_reason: "General availability".to_string(),
                }));
            }
        }
        
        // Find agents with required capabilities
        let mut candidates = Vec::new();
        
        for capability in &required_caps {
            let matches = self.repository
                .find_agents_by_capability(capability, 20)
                .await?;
            
            for agent_match in matches {
                candidates.push((agent_match.agent_name, capability.clone()));
            }
        }
        
        // Score each candidate
        let mut agent_scores: HashMap<String, AgentScore> = HashMap::new();
        
        for (agent_name, capability) in candidates {
            let agent = self.repository.get_agent(&agent_name).await?
                .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", agent_name)))?;
            
            if !agent.is_available() {
                continue;
            }
            
            let entry = agent_scores.entry(agent_name.clone())
                .or_insert(AgentScore {
                    agent_name: agent_name.clone(),
                    matched_capabilities: vec![],
                    total_score: 0.0,
                    agent_profile: agent.clone(),
                });
            
            entry.matched_capabilities.push(capability);
        }
        
        // Calculate final scores
        let mut recommendations = Vec::new();
        
        for (agent_name, mut score) in agent_scores {
            let agent = &score.agent_profile;
            
            // Capability match score
            let capability_match = score.matched_capabilities.len() as f64 / required_caps.len() as f64;
            
            // Skip if doesn't meet minimum capability requirements
            if capability_match < 0.5 {
                continue;
            }
            
            // Load factor (prefer less loaded agents)
            let load_factor = 1.0 - (agent.current_load as f64 / agent.max_concurrent_tasks as f64);
            
            // Specialization bonus
            let specialization_bonus = score.matched_capabilities.iter()
                .filter(|cap| agent.specializations.contains(cap))
                .count() as f64 * 0.2;
            
            // Calculate final score
            let total_score = capability_match * 0.4 +
                             load_factor * 0.3 +
                             agent.reputation_score * 0.2 +
                             specialization_bonus * 0.1;
            
            recommendations.push(AgentRecommendation {
                agent_name: agent.name.clone(),
                match_score: total_score,
                capability_matches: score.matched_capabilities,
                load_factor,
                specialization_bonus,
                recommendation_reason: if specialization_bonus > 0.0 {
                    "Specialist in required capabilities".to_string()
                } else {
                    "Has required capabilities".to_string()
                },
            });
        }
        
        // Sort by score and return best
        recommendations.sort_by(|a, b| {
            b.match_score.partial_cmp(&a.match_score).unwrap()
        });
        
        Ok(recommendations.into_iter().next())
    }
    
    /// Find agents for team formation
    pub async fn find_team_for_capabilities(
        &self,
        required_capabilities: Vec<String>,
        team_size_limit: i32,
    ) -> Result<Vec<AgentProfile>> {
        let mut selected_agents = Vec::new();
        let mut covered_capabilities = std::collections::HashSet::new();
        
        // Get all available agents
        let mut agents = self.repository.list_agents(
            AgentFilter::available_only()
        ).await?;
        
        // Sort by reputation and capability count
        agents.sort_by(|a, b| {
            let a_score = a.reputation_score * (a.capabilities.len() as f64);
            let b_score = b.reputation_score * (b.capabilities.len() as f64);
            b_score.partial_cmp(&a_score).unwrap()
        });
        
        // Greedy selection to cover all capabilities
        for agent in agents {
            if selected_agents.len() >= team_size_limit as usize {
                break;
            }
            
            // Check if this agent adds new capabilities
            let new_caps: Vec<_> = agent.capabilities.iter()
                .filter(|cap| required_capabilities.contains(cap) && !covered_capabilities.contains(*cap))
                .collect();
            
            if !new_caps.is_empty() {
                for cap in &new_caps {
                    covered_capabilities.insert((*cap).clone());
                }
                selected_agents.push(agent);
                
                // Check if we've covered all requirements
                if covered_capabilities.len() == required_capabilities.len() {
                    break;
                }
            }
        }
        
        Ok(selected_agents)
    }
}

#[derive(Debug, Clone)]
struct AgentScore {
    agent_name: String,
    matched_capabilities: Vec<String>,
    total_score: f64,
    agent_profile: AgentProfile,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRecommendation {
    pub agent_name: String,
    pub match_score: f64,
    pub capability_matches: Vec<String>,
    pub load_factor: f64,
    pub specialization_bonus: f64,
    pub recommendation_reason: String,
}
```

## Files to Create/Modify
- `mcp-protocol/src/handler.rs` - Add agent handler methods
- `mcp-protocol/src/params.rs` - Add agent parameter types
- `mcp-protocol/src/services/agent_monitor.rs` - Agent monitoring service
- `mcp-protocol/src/services/agent_discovery.rs` - Agent discovery service
- `mcp-protocol/src/router.rs` - Add agent method routing

## Testing Requirements
1. Test agent registration with validation
2. Test capability-based discovery
3. Test heartbeat and timeout handling
4. Test status transitions
5. Test load balancing recommendations
6. Test team formation
7. Test concurrent agent operations

## Notes
- Heartbeat timeout triggers offline status
- Status transitions are validated
- Load balancing recommendations provided
- Team formation for multi-capability requirements
- Specialization bonuses in agent selection