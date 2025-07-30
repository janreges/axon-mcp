# DATABASE11: Create Work Discovery Functions

## Objective
Implement optimized work discovery functions and stored queries that enable agents to efficiently find tasks matching their capabilities, with support for long-polling and intelligent task routing.

## Implementation Details

### 1. Create Work Discovery Views
Create `database/migrations/sqlite/008_work_discovery_views.sql`:

```sql
-- View for available tasks with capability requirements
CREATE VIEW IF NOT EXISTS available_tasks AS
SELECT 
    t.id,
    t.code,
    t.name,
    t.description,
    t.state,
    t.priority_score,
    t.failure_count,
    t.inserted_at,
    t.required_capabilities,
    t.confidence_threshold,
    t.owner_agent_name,
    t.parent_task_id,
    t.workflow_definition_id,
    t.workflow_cursor,
    -- Calculate task age in minutes
    (julianday('now') - julianday(t.inserted_at)) * 24 * 60 as age_minutes,
    -- Check if task has pending handoff
    EXISTS(
        SELECT 1 FROM handoffs h 
        WHERE h.task_code = t.code 
          AND h.accepted_at IS NULL
    ) as has_pending_handoff,
    -- Count unresolved blockers
    (
        SELECT COUNT(*) 
        FROM help_requests hr
        WHERE hr.task_code = t.code 
          AND hr.help_type = 'blocker'
          AND hr.resolved_at IS NULL
    ) as unresolved_blockers
FROM tasks t
WHERE t.state IN ('Created', 'InProgress', 'Review', 'PendingHandoff', 'Blocked')
  AND (t.state != 'Blocked' OR EXISTS(
      SELECT 1 FROM help_requests hr2
      WHERE hr2.task_code = t.code 
        AND hr2.help_type = 'blocker'
        AND hr2.resolved_at IS NOT NULL
        AND hr2.resolved_at > datetime('now', '-1 hour')
  ));

-- View for task-agent capability matching
CREATE VIEW IF NOT EXISTS task_agent_matches AS
WITH task_capabilities AS (
    SELECT 
        t.code as task_code,
        json_each.value as required_capability
    FROM tasks t, 
         json_each(CASE 
             WHEN t.required_capabilities IS NOT NULL 
               AND t.required_capabilities != '[]' 
             THEN t.required_capabilities 
             ELSE '[]' 
         END)
    WHERE t.state IN ('Created', 'PendingHandoff')
),
agent_capabilities AS (
    SELECT 
        a.name as agent_name,
        a.current_load,
        a.max_concurrent_tasks,
        a.reputation_score,
        a.status,
        json_each.value as capability
    FROM agents a, json_each(a.capabilities)
    WHERE a.status IN ('idle', 'active')
      AND a.current_load < a.max_concurrent_tasks
)
SELECT DISTINCT
    tc.task_code,
    ac.agent_name,
    COUNT(DISTINCT tc.required_capability) as matched_capabilities,
    ac.current_load,
    ac.max_concurrent_tasks - ac.current_load as available_capacity,
    ac.reputation_score,
    ac.status
FROM task_capabilities tc
INNER JOIN agent_capabilities ac ON tc.required_capability = ac.capability
GROUP BY tc.task_code, ac.agent_name;

-- View for handoff opportunities
CREATE VIEW IF NOT EXISTS handoff_opportunities AS
SELECT 
    h.id as handoff_id,
    h.task_code,
    h.from_agent_name,
    h.to_capability,
    h.confidence_score,
    h.created_at,
    h.estimated_effort,
    t.name as task_name,
    t.priority_score,
    t.state as task_state,
    a.name as suitable_agent,
    a.reputation_score as agent_reputation,
    a.current_load as agent_load
FROM handoffs h
INNER JOIN tasks t ON t.code = h.task_code
INNER JOIN agents a ON EXISTS (
    SELECT 1 FROM json_each(a.capabilities)
    WHERE json_each.value = h.to_capability
)
WHERE h.accepted_at IS NULL
  AND t.state = 'PendingHandoff'
  AND a.status IN ('idle', 'active')
  AND a.current_load < a.max_concurrent_tasks
ORDER BY 
    h.confidence_score DESC,
    t.priority_score DESC,
    a.reputation_score DESC,
    a.current_load ASC;

-- View for work queue with scoring
CREATE VIEW IF NOT EXISTS work_queue AS
SELECT 
    t.*,
    -- Calculate composite score for task prioritization
    (
        -- Priority component (0-100)
        t.priority_score * 10 +
        -- Age component (older tasks get higher score)
        MIN(100, (julianday('now') - julianday(t.inserted_at)) * 24 * 2) +
        -- State urgency component
        CASE t.state
            WHEN 'Blocked' THEN 20
            WHEN 'Review' THEN 15
            WHEN 'PendingHandoff' THEN 10
            WHEN 'InProgress' THEN 5
            WHEN 'Created' THEN 0
        END +
        -- Failure penalty (reduce score for repeatedly failing tasks)
        MAX(-50, -t.failure_count * 10) +
        -- Parent task bonus (subtasks get priority)
        CASE WHEN t.parent_task_id IS NOT NULL THEN 10 ELSE 0 END
    ) as urgency_score,
    -- Get best matching agent
    (
        SELECT json_object(
            'agent_name', m.agent_name,
            'match_score', CAST(m.matched_capabilities AS REAL) / 
                          CAST(json_array_length(t.required_capabilities) AS REAL),
            'reputation', m.reputation_score
        )
        FROM task_agent_matches m
        WHERE m.task_code = t.code
        ORDER BY 
            CAST(m.matched_capabilities AS REAL) / 
            CAST(json_array_length(t.required_capabilities) AS REAL) DESC,
            m.reputation_score DESC,
            m.current_load ASC
        LIMIT 1
    ) as best_agent_match
FROM available_tasks t
ORDER BY urgency_score DESC;
```

### 2. Create Work Discovery Implementation
In `database/src/work_discovery.rs`:

```rust
use crate::{TaskRepository, Task, WorkDiscoveryParams, Result, TaskError};
use chrono::{Utc, Duration};
use sqlx::{SqlitePool, Row};
use tokio::time::{sleep, timeout, Duration as TokioDuration};

/// Enhanced work discovery with intelligent routing
pub struct WorkDiscoveryEngine {
    pool: SqlitePool,
}

impl WorkDiscoveryEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    
    /// Discover work with capability matching and scoring
    pub async fn discover_work_enhanced(
        &self,
        agent_name: &str,
        capabilities: Vec<String>,
        max_tasks: i32,
    ) -> Result<Vec<TaskWithScore>> {
        let capabilities_json = serde_json::to_string(&capabilities)
            .map_err(|e| TaskError::Serialization(format!("Failed to serialize capabilities: {}", e)))?;
        
        let tasks = sqlx::query(
            r#"
            WITH agent_work AS (
                SELECT 
                    t.*,
                    wq.urgency_score,
                    wq.best_agent_match,
                    -- Calculate capability match score
                    CASE 
                        WHEN t.required_capabilities IS NULL OR t.required_capabilities = '[]' 
                        THEN 1.0
                        ELSE (
                            SELECT CAST(COUNT(*) AS REAL) / CAST(json_array_length(t.required_capabilities) AS REAL)
                            FROM json_each(t.required_capabilities) req
                            WHERE EXISTS (
                                SELECT 1 FROM json_each(?) cap
                                WHERE cap.value = req.value
                            )
                        )
                    END as capability_match_score,
                    -- Check if agent is best match
                    CASE 
                        WHEN json_extract(wq.best_agent_match, '$.agent_name') = ?
                        THEN 1 
                        ELSE 0 
                    END as is_best_match
                FROM work_queue wq
                INNER JOIN tasks t ON t.code = wq.code
                WHERE 
                    -- Basic filters
                    t.owner_agent_name = 'unassigned' OR t.state = 'PendingHandoff'
                    -- Capability check
                    AND (
                        t.required_capabilities IS NULL 
                        OR t.required_capabilities = '[]'
                        OR EXISTS (
                            SELECT 1 FROM json_each(t.required_capabilities) req
                            WHERE EXISTS (
                                SELECT 1 FROM json_each(?) cap
                                WHERE cap.value = req.value
                            )
                        )
                    )
                    -- Exclude tasks agent recently failed
                    AND NOT EXISTS (
                        SELECT 1 FROM system_events e
                        WHERE e.task_code = t.code
                          AND e.actor_id = ?
                          AND e.event_type = 'task_failed'
                          AND e.timestamp > datetime('now', '-24 hours')
                    )
            )
            SELECT *
            FROM agent_work
            ORDER BY 
                is_best_match DESC,
                urgency_score DESC,
                capability_match_score DESC
            LIMIT ?
            "#
        )
        .bind(&capabilities_json)
        .bind(agent_name)
        .bind(&capabilities_json)
        .bind(agent_name)
        .bind(max_tasks)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut results = Vec::new();
        for row in tasks {
            let task = row_to_task(row)?;
            let urgency_score: f64 = row.get("urgency_score");
            let capability_match_score: f64 = row.get("capability_match_score");
            let is_best_match: bool = row.get::<i32, _>("is_best_match") == 1;
            
            results.push(TaskWithScore {
                task,
                urgency_score,
                capability_match_score,
                is_best_match,
            });
        }
        
        Ok(results)
    }
    
    /// Long-polling work discovery
    pub async fn discover_work_long_poll(
        &self,
        agent_name: &str,
        capabilities: Vec<String>,
        timeout_seconds: u64,
        poll_interval_seconds: u64,
    ) -> Result<Vec<Task>> {
        let deadline = Utc::now() + Duration::seconds(timeout_seconds as i64);
        let poll_interval = TokioDuration::from_secs(poll_interval_seconds);
        
        // Record agent as waiting for work
        self.update_agent_waiting_status(agent_name, true).await?;
        
        let result = timeout(
            TokioDuration::from_secs(timeout_seconds),
            async {
                loop {
                    // Try to discover work
                    let tasks = self.discover_work_enhanced(agent_name, capabilities.clone(), 5).await?;
                    
                    if !tasks.is_empty() {
                        // Found work!
                        return Ok(tasks.into_iter().map(|t| t.task).collect());
                    }
                    
                    // Check if we should continue polling
                    if Utc::now() >= deadline {
                        break;
                    }
                    
                    // Sleep before next poll
                    sleep(poll_interval).await;
                }
                
                Ok(vec![])
            }
        ).await;
        
        // Clear waiting status
        self.update_agent_waiting_status(agent_name, false).await?;
        
        match result {
            Ok(inner_result) => inner_result,
            Err(_) => Ok(vec![]), // Timeout is not an error, just return empty
        }
    }
    
    /// Get work recommendations based on agent specializations
    pub async fn get_work_recommendations(
        &self,
        agent_name: &str,
        limit: i32,
    ) -> Result<Vec<WorkRecommendation>> {
        // Get agent profile
        let agent = sqlx::query(
            "SELECT capabilities, specializations, reputation_score FROM agents WHERE name = ?"
        )
        .bind(agent_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", agent_name)))?;
        
        let capabilities: String = agent.get("capabilities");
        let specializations: String = agent.get("specializations");
        let reputation_score: f64 = agent.get("reputation_score");
        
        // Find tasks matching specializations
        let recommendations = sqlx::query(
            r#"
            SELECT 
                t.code,
                t.name,
                t.required_capabilities,
                wq.urgency_score,
                'specialization_match' as reason,
                1.0 as confidence
            FROM work_queue wq
            INNER JOIN tasks t ON t.code = wq.code
            WHERE EXISTS (
                SELECT 1 FROM json_each(?) spec
                WHERE EXISTS (
                    SELECT 1 FROM json_each(t.required_capabilities) req
                    WHERE req.value = spec.value
                )
            )
            AND (t.owner_agent_name = 'unassigned' OR t.state = 'PendingHandoff')
            
            UNION ALL
            
            -- Tasks where agent is best match
            SELECT 
                t.code,
                t.name,
                t.required_capabilities,
                wq.urgency_score,
                'best_match' as reason,
                0.9 as confidence
            FROM work_queue wq
            INNER JOIN tasks t ON t.code = wq.code
            WHERE json_extract(wq.best_agent_match, '$.agent_name') = ?
            AND (t.owner_agent_name = 'unassigned' OR t.state = 'PendingHandoff')
            
            UNION ALL
            
            -- High priority tasks agent can handle
            SELECT 
                t.code,
                t.name,
                t.required_capabilities,
                wq.urgency_score,
                'high_priority' as reason,
                0.8 as confidence
            FROM work_queue wq
            INNER JOIN tasks t ON t.code = wq.code
            WHERE wq.urgency_score > 150
            AND (
                t.required_capabilities IS NULL 
                OR t.required_capabilities = '[]'
                OR EXISTS (
                    SELECT 1 FROM json_each(t.required_capabilities) req
                    WHERE EXISTS (
                        SELECT 1 FROM json_each(?) cap
                        WHERE cap.value = req.value
                    )
                )
            )
            AND (t.owner_agent_name = 'unassigned' OR t.state = 'PendingHandoff')
            
            ORDER BY confidence DESC, urgency_score DESC
            LIMIT ?
            "#
        )
        .bind(&specializations)
        .bind(agent_name)
        .bind(&capabilities)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut results = Vec::new();
        for row in recommendations {
            results.push(WorkRecommendation {
                task_code: row.get("code"),
                task_name: row.get("name"),
                reason: row.get("reason"),
                confidence: row.get("confidence"),
                urgency_score: row.get("urgency_score"),
            });
        }
        
        Ok(results)
    }
    
    /// Update agent waiting status for work discovery
    async fn update_agent_waiting_status(
        &self,
        agent_name: &str,
        is_waiting: bool,
    ) -> Result<()> {
        // This could update a "waiting_for_work" flag or timestamp
        // For now, just log an event
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: if is_waiting { 
                "agent_waiting_for_work".to_string() 
            } else { 
                "agent_stopped_waiting".to_string() 
            },
            actor_type: ActorType::Agent,
            actor_id: agent_name.to_string(),
            task_code: None,
            payload: serde_json::json!({}),
            correlation_id: None,
        };
        
        // Log event (would need access to repository)
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TaskWithScore {
    pub task: Task,
    pub urgency_score: f64,
    pub capability_match_score: f64,
    pub is_best_match: bool,
}

#[derive(Debug, Clone)]
pub struct WorkRecommendation {
    pub task_code: String,
    pub task_name: String,
    pub reason: String,
    pub confidence: f64,
    pub urgency_score: f64,
}
```

### 3. Create Circuit Breaker for Failed Tasks
In `database/src/circuit_breaker.rs`:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};

/// Circuit breaker to prevent repeatedly assigning failing tasks
pub struct TaskCircuitBreaker {
    failures: Arc<RwLock<HashMap<String, TaskFailureInfo>>>,
    failure_threshold: u32,
    reset_timeout: Duration,
}

#[derive(Debug, Clone)]
struct TaskFailureInfo {
    failure_count: u32,
    last_failure: DateTime<Utc>,
    failing_agents: Vec<String>,
}

impl TaskCircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout_minutes: i64) -> Self {
        Self {
            failures: Arc::new(RwLock::new(HashMap::new())),
            failure_threshold,
            reset_timeout: Duration::minutes(reset_timeout_minutes),
        }
    }
    
    /// Record a task failure
    pub async fn record_failure(&self, task_code: &str, agent_name: &str) {
        let mut failures = self.failures.write().await;
        
        let entry = failures.entry(task_code.to_string()).or_insert(TaskFailureInfo {
            failure_count: 0,
            last_failure: Utc::now(),
            failing_agents: Vec::new(),
        });
        
        entry.failure_count += 1;
        entry.last_failure = Utc::now();
        if !entry.failing_agents.contains(&agent_name.to_string()) {
            entry.failing_agents.push(agent_name.to_string());
        }
    }
    
    /// Check if task is available (not in failed state)
    pub async fn is_task_available(&self, task_code: &str) -> bool {
        let failures = self.failures.read().await;
        
        if let Some(info) = failures.get(task_code) {
            // Check if we should reset the circuit breaker
            if Utc::now() - info.last_failure > self.reset_timeout {
                return true;
            }
            
            // Check if threshold exceeded
            if info.failure_count >= self.failure_threshold {
                return false;
            }
        }
        
        true
    }
    
    /// Get tasks that should be quarantined
    pub async fn get_quarantined_tasks(&self) -> Vec<String> {
        let failures = self.failures.read().await;
        
        failures.iter()
            .filter(|(_, info)| {
                info.failure_count >= self.failure_threshold &&
                Utc::now() - info.last_failure <= self.reset_timeout
            })
            .map(|(code, _)| code.clone())
            .collect()
    }
    
    /// Reset circuit breaker for a task
    pub async fn reset_task(&self, task_code: &str) {
        let mut failures = self.failures.write().await;
        failures.remove(task_code);
    }
    
    /// Clean up old entries
    pub async fn cleanup(&self) {
        let mut failures = self.failures.write().await;
        let now = Utc::now();
        
        failures.retain(|_, info| {
            now - info.last_failure <= self.reset_timeout * 2
        });
    }
}
```

### 4. Create Batch Work Assignment
In `database/src/work_assignment.rs`:

```rust
use crate::{TaskRepository, Task, AgentProfile, Result};
use std::collections::HashMap;

/// Batch work assignment for optimal distribution
pub struct WorkAssignmentEngine<R: TaskRepository> {
    repository: R,
}

impl<R: TaskRepository> WorkAssignmentEngine<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
    
    /// Assign multiple tasks to multiple agents optimally
    pub async fn batch_assign_work(&self) -> Result<Vec<WorkAssignment>> {
        // Get all available agents
        let agents = self.repository.list_agents(AgentFilter::available_only()).await?;
        
        // Get all unassigned tasks
        let tasks = self.repository.discover_work(WorkDiscoveryParams {
            agent_name: "system".to_string(),
            capabilities: vec![], // Get all tasks
            max_tasks: 100,
            include_types: vec![TaskState::Created, TaskState::PendingHandoff],
            exclude_codes: vec![],
            min_priority: None,
        }).await?;
        
        // Build capability index
        let mut capability_agents: HashMap<String, Vec<&AgentProfile>> = HashMap::new();
        for agent in &agents {
            for capability in &agent.capabilities {
                capability_agents.entry(capability.clone())
                    .or_insert_with(Vec::new)
                    .push(agent);
            }
        }
        
        // Sort agents by capability for each capability
        for agents in capability_agents.values_mut() {
            agents.sort_by(|a, b| {
                b.reputation_score.partial_cmp(&a.reputation_score)
                    .unwrap()
                    .then_with(|| a.current_load.cmp(&b.current_load))
            });
        }
        
        let mut assignments = Vec::new();
        let mut agent_loads: HashMap<String, i32> = HashMap::new();
        
        // Initialize current loads
        for agent in &agents {
            agent_loads.insert(agent.name.clone(), agent.current_load);
        }
        
        // Assign tasks
        for task in tasks {
            // Parse required capabilities
            let required_caps = if let Some(caps_json) = &task.required_capabilities {
                serde_json::from_str::<Vec<String>>(caps_json).unwrap_or_default()
            } else {
                vec![]
            };
            
            // Find best available agent
            let mut best_agent: Option<&AgentProfile> = None;
            let mut best_score = 0.0;
            
            for agent in &agents {
                let current_load = agent_loads.get(&agent.name).copied().unwrap_or(0);
                
                // Skip if at capacity
                if current_load >= agent.max_concurrent_tasks {
                    continue;
                }
                
                // Calculate match score
                let capability_match = if required_caps.is_empty() {
                    1.0
                } else {
                    let matched = required_caps.iter()
                        .filter(|cap| agent.has_capability(cap))
                        .count();
                    matched as f64 / required_caps.len() as f64
                };
                
                // Skip if doesn't meet minimum capability match
                if capability_match < 0.5 {
                    continue;
                }
                
                // Calculate overall score
                let load_factor = 1.0 - (current_load as f64 / agent.max_concurrent_tasks as f64);
                let score = capability_match * 0.5 + 
                           agent.reputation_score * 0.3 + 
                           load_factor * 0.2;
                
                if score > best_score {
                    best_score = score;
                    best_agent = Some(agent);
                }
            }
            
            // Create assignment if agent found
            if let Some(agent) = best_agent {
                assignments.push(WorkAssignment {
                    task_code: task.code.clone(),
                    agent_name: agent.name.clone(),
                    confidence_score: best_score,
                    assignment_reason: if required_caps.is_empty() {
                        "general_availability".to_string()
                    } else {
                        "capability_match".to_string()
                    },
                });
                
                // Update load tracking
                *agent_loads.get_mut(&agent.name).unwrap() += 1;
            }
        }
        
        Ok(assignments)
    }
}

#[derive(Debug, Clone)]
pub struct WorkAssignment {
    pub task_code: String,
    pub agent_name: String,
    pub confidence_score: f64,
    pub assignment_reason: String,
}
```

## Files to Create/Modify
- `database/migrations/sqlite/008_work_discovery_views.sql` - New views
- `database/src/work_discovery.rs` - New work discovery engine
- `database/src/circuit_breaker.rs` - New circuit breaker implementation
- `database/src/work_assignment.rs` - New batch assignment engine
- `database/src/lib.rs` - Export new modules

## Testing Requirements
1. Test work discovery with various capability combinations
2. Test long-polling functionality
3. Test circuit breaker with failing tasks
4. Test batch assignment optimization
5. Test work recommendations
6. Performance test with many agents and tasks
7. Test view queries performance

## Performance Considerations
1. Views pre-calculate common queries
2. Indexes on JSON fields for capability matching
3. Long-polling reduces database load
4. Circuit breaker prevents retry storms
5. Batch assignment minimizes individual queries
6. Consider materialized views for very large scale

## Usage Examples

### Long-Polling Work Discovery
```rust
let engine = WorkDiscoveryEngine::new(pool);
let tasks = engine.discover_work_long_poll(
    "rust-architect",
    vec!["rust".to_string(), "architecture".to_string()],
    120, // 2 minute timeout
    3,   // 3 second poll interval
).await?;
```

### Get Work Recommendations
```rust
let recommendations = engine.get_work_recommendations("frontend-dev", 5).await?;
for rec in recommendations {
    println!("{}: {} (confidence: {})", rec.task_code, rec.reason, rec.confidence);
}
```

### Batch Assignment
```rust
let assignment_engine = WorkAssignmentEngine::new(repository);
let assignments = assignment_engine.batch_assign_work().await?;
for assignment in assignments {
    println!("Assign {} to {} (score: {})", 
        assignment.task_code, 
        assignment.agent_name, 
        assignment.confidence_score
    );
}
```