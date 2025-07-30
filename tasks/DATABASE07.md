# DATABASE07: Implement Agent Management Repository

## Objective
Implement all agent management methods in the SQLite repository, including registration, workload tracking, capability matching, and heartbeat management.

## Implementation Details

### 1. Add Agent Methods to SqliteTaskRepository
In `database/src/sqlite.rs`, add implementations for agent-related methods:

```rust
impl TaskRepository for SqliteTaskRepository {
    // ... existing implementations ...
    
    async fn register_agent(&self, agent: NewAgent) -> Result<AgentProfile> {
        // Validate input
        agent.validate()?;
        
        // Check if agent name already exists
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE name = ?)"
        )
        .bind(&agent.name)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if exists {
            return Err(TaskError::AlreadyExists(
                format!("Agent {} already registered", agent.name)
            ));
        }
        
        // Convert arrays to JSON
        let capabilities_json = capabilities_to_json(&agent.capabilities);
        let specializations_json = capabilities_to_json(&agent.specializations);
        let preferences_json = agent.preferences
            .as_ref()
            .map(|p| serde_json::to_string(p).unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|| "{}".to_string());
        
        // Insert agent
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO agents 
            (name, description, capabilities, max_concurrent_tasks, 
             registered_by, specializations, preferences, status, current_load)
            VALUES (?, ?, ?, ?, ?, ?, ?, 'idle', 0)
            RETURNING id
            "#
        )
        .bind(&agent.name)
        .bind(&agent.description)
        .bind(&capabilities_json)
        .bind(agent.max_concurrent_tasks)
        .bind(&agent.registered_by)
        .bind(&specializations_json)
        .bind(&preferences_json)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Fetch and return created agent
        self.get_agent(&agent.name)
            .await?
            .ok_or_else(|| TaskError::Database("Failed to fetch created agent".to_string()))
    }
    
    async fn get_agent(&self, agent_name: &str) -> Result<Option<AgentProfile>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, capabilities, max_concurrent_tasks,
                   current_load, status, preferences, last_heartbeat,
                   reputation_score, specializations, registered_at, registered_by
            FROM agents
            WHERE name = ?
            "#
        )
        .bind(agent_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        row.map(|r| self.row_to_agent_profile(r)).transpose()
    }
    
    async fn list_agents(&self, filter: AgentFilter) -> Result<Vec<AgentProfile>> {
        let mut query = QueryBuilder::new(
            r#"
            SELECT id, name, description, capabilities, max_concurrent_tasks,
                   current_load, status, preferences, last_heartbeat,
                   reputation_score, specializations, registered_at, registered_by
            FROM agents
            WHERE 1=1
            "#
        );
        
        // Apply filters
        if let Some(status) = &filter.status {
            query.push(" AND status = ");
            query.push_bind(status.to_string());
        }
        
        if let Some(capability) = &filter.has_capability {
            query.push(" AND EXISTS (");
            query.push("SELECT 1 FROM json_each(capabilities) WHERE value = ");
            query.push_bind(capability);
            query.push(")");
        }
        
        if filter.has_capacity {
            query.push(" AND current_load < max_concurrent_tasks");
            query.push(" AND status IN ('idle', 'active')");
        }
        
        if let Some(min_rep) = filter.min_reputation {
            query.push(" AND reputation_score >= ");
            query.push_bind(min_rep);
        }
        
        if let Some(spec) = &filter.specialization {
            query.push(" AND EXISTS (");
            query.push("SELECT 1 FROM json_each(specializations) WHERE value = ");
            query.push_bind(spec);
            query.push(")");
        }
        
        // Order by reputation and load
        query.push(" ORDER BY reputation_score DESC, current_load ASC");
        
        // Apply pagination
        if let Some(limit) = filter.limit {
            query.push(" LIMIT ");
            query.push_bind(limit);
        }
        
        if let Some(offset) = filter.offset {
            query.push(" OFFSET ");
            query.push_bind(offset);
        }
        
        let agents = query.build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        agents.into_iter()
            .map(|row| self.row_to_agent_profile(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn update_agent_status(&self, agent_name: &str, status: AgentStatus) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE agents SET status = ?, last_heartbeat = CURRENT_TIMESTAMP WHERE name = ?"
        )
        .bind(status.to_string())
        .bind(agent_name)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .rows_affected();
        
        if affected == 0 {
            return Err(TaskError::NotFound(format!("Agent {} not found", agent_name)));
        }
        
        Ok(())
    }
    
    async fn heartbeat(&self, agent_name: &str) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE agents SET last_heartbeat = CURRENT_TIMESTAMP WHERE name = ?"
        )
        .bind(agent_name)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .rows_affected();
        
        if affected == 0 {
            return Err(TaskError::NotFound(format!("Agent {} not found", agent_name)));
        }
        
        // Check if agent was unresponsive and update status if needed
        sqlx::query(
            r#"
            UPDATE agents 
            SET status = 'active' 
            WHERE name = ? 
              AND status = 'unresponsive'
              AND current_load > 0
            "#
        )
        .bind(agent_name)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Or set to idle if no load
        sqlx::query(
            r#"
            UPDATE agents 
            SET status = 'idle' 
            WHERE name = ? 
              AND status = 'unresponsive'
              AND current_load = 0
            "#
        )
        .bind(agent_name)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(())
    }
    
    async fn update_agent_load(&self, agent_name: &str, current_load: i32) -> Result<()> {
        // This will trigger validation in the database
        let result = sqlx::query(
            r#"
            UPDATE agents 
            SET current_load = ?,
                status = CASE
                    WHEN ? = 0 AND status = 'active' THEN 'idle'
                    WHEN ? > 0 AND status = 'idle' THEN 'active'
                    ELSE status
                END
            WHERE name = ?
            "#
        )
        .bind(current_load)
        .bind(current_load)
        .bind(current_load)
        .bind(agent_name)
        .execute(&self.pool)
        .await;
        
        match result {
            Ok(result) if result.rows_affected() == 0 => {
                Err(TaskError::NotFound(format!("Agent {} not found", agent_name)))
            }
            Ok(_) => Ok(()),
            Err(e) if e.to_string().contains("Current load cannot exceed") => {
                Err(TaskError::Validation("Current load exceeds max concurrent tasks".to_string()))
            }
            Err(e) => Err(sqlx_error_to_task_error(e)),
        }
    }
    
    async fn find_agents_by_capability(&self, capability: &str, limit: i32) -> Result<Vec<AgentCapabilityMatch>> {
        let agents = sqlx::query(
            r#"
            SELECT 
                name,
                capabilities,
                specializations,
                current_load,
                max_concurrent_tasks,
                reputation_score,
                status
            FROM agents
            WHERE status IN ('idle', 'active')
              AND current_load < max_concurrent_tasks
              AND EXISTS (
                  SELECT 1 FROM json_each(capabilities)
                  WHERE value = ?
              )
            ORDER BY 
                reputation_score DESC,
                current_load ASC
            LIMIT ?
            "#
        )
        .bind(capability)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut matches = Vec::new();
        
        for row in agents {
            let name: String = row.get("name");
            let caps_json: String = row.get("capabilities");
            let specs_json: String = row.get("specializations");
            let current_load: i32 = row.get("current_load");
            let reputation_score: f64 = row.get("reputation_score");
            
            let capabilities = parse_capabilities(&caps_json)?;
            let specializations = parse_capabilities(&specs_json)?;
            
            let is_specialized = specializations.contains(&capability.to_string());
            
            matches.push(AgentCapabilityMatch {
                agent_name: name,
                match_score: 1.0, // Has the capability
                matched_capabilities: vec![capability.to_string()],
                missing_capabilities: vec![],
                is_specialized,
                current_load,
                reputation_score,
            });
        }
        
        Ok(matches)
    }
    
    async fn get_agent_workload(&self, agent_name: &str) -> Result<AgentWorkloadSummary> {
        // Get agent basic info
        let agent = self.get_agent(agent_name)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", agent_name)))?;
        
        // Get active tasks
        let active_tasks = sqlx::query(
            r#"
            SELECT code, name, state, inserted_at
            FROM tasks
            WHERE owner_agent_name = ?
              AND state IN ('InProgress', 'Review', 'Blocked', 'PendingHandoff')
            ORDER BY inserted_at DESC
            "#
        )
        .bind(agent_name)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let task_summaries: Vec<TaskSummary> = active_tasks.into_iter()
            .map(|row| {
                let state_str: String = row.get("state");
                let state = TaskState::try_from(state_str.as_str())
                    .unwrap_or(TaskState::Created);
                
                TaskSummary {
                    code: row.get("code"),
                    name: row.get("name"),
                    state,
                    started_at: row.get("inserted_at"),
                    estimated_effort: None, // Would need to join with other tables
                }
            })
            .collect();
        
        // Get today's completed count
        let completed_today = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COUNT(*)
            FROM tasks
            WHERE owner_agent_name = ?
              AND state = 'Done'
              AND done_at >= date('now', 'start of day')
            "#
        )
        .bind(agent_name)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Get average duration from work sessions
        let avg_duration = sqlx::query_scalar::<_, Option<f64>>(
            r#"
            SELECT AVG(total_active_minutes)
            FROM work_sessions
            WHERE agent_name = ?
              AND finished_at IS NOT NULL
              AND finished_at >= datetime('now', '-7 days')
            "#
        )
        .bind(agent_name)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .unwrap_or(0.0);
        
        // Get last completed task time
        let last_completed = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            r#"
            SELECT MAX(done_at)
            FROM tasks
            WHERE owner_agent_name = ?
              AND state = 'Done'
            "#
        )
        .bind(agent_name)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(AgentWorkloadSummary {
            agent_name: agent_name.to_string(),
            status: agent.status,
            active_tasks: task_summaries,
            completed_today,
            average_task_duration_minutes: avg_duration,
            current_load_percentage: agent.load_percentage(),
            last_task_completed_at: last_completed,
        })
    }
    
    async fn update_agent_reputation(&self, agent_name: &str, delta: f64) -> Result<()> {
        // Update reputation with bounds checking (trigger will clamp)
        sqlx::query(
            r#"
            UPDATE agents 
            SET reputation_score = reputation_score + ?
            WHERE name = ?
            "#
        )
        .bind(delta)
        .bind(agent_name)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(())
    }
    
    async fn deactivate_agent(&self, agent_name: &str) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE agents SET status = 'offline' WHERE name = ?"
        )
        .bind(agent_name)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .rows_affected();
        
        if affected == 0 {
            return Err(TaskError::NotFound(format!("Agent {} not found", agent_name)));
        }
        
        Ok(())
    }
    
    async fn get_unresponsive_agents(&self, timeout_seconds: i64) -> Result<Vec<AgentProfile>> {
        let agents = sqlx::query(
            r#"
            SELECT id, name, description, capabilities, max_concurrent_tasks,
                   current_load, status, preferences, last_heartbeat,
                   reputation_score, specializations, registered_at, registered_by
            FROM agents
            WHERE status IN ('idle', 'active', 'blocked')
              AND (strftime('%s', 'now') - strftime('%s', last_heartbeat)) > ?
            "#
        )
        .bind(timeout_seconds)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        agents.into_iter()
            .map(|row| self.row_to_agent_profile(row))
            .collect::<Result<Vec<_>>>()
    }
}
```

### 2. Add Work Discovery Implementation
```rust
impl TaskRepository for SqliteTaskRepository {
    async fn discover_work(&self, params: WorkDiscoveryParams) -> Result<Vec<Task>> {
        // Get agent capabilities
        let agent = self.get_agent(&params.agent_name)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", params.agent_name)))?;
        
        let mut query = QueryBuilder::new(
            r#"
            SELECT DISTINCT t.*
            FROM tasks t
            WHERE t.state IN (
            "#
        );
        
        // Add states
        let mut separated = query.separated(", ");
        for state in &params.include_types {
            separated.push_bind(state.to_string());
        }
        query.push(")");
        
        // Exclude specific task codes
        if !params.exclude_codes.is_empty() {
            query.push(" AND t.code NOT IN (");
            let mut separated = query.separated(", ");
            for code in &params.exclude_codes {
                separated.push_bind(code);
            }
            query.push(")");
        }
        
        // Match capabilities if task has requirements
        query.push(r#"
            AND (
                t.required_capabilities IS NULL 
                OR t.required_capabilities = '[]'
                OR EXISTS (
                    SELECT 1 FROM json_each(t.required_capabilities) AS req
                    WHERE EXISTS (
                        SELECT 1 FROM json_each(?) AS cap
                        WHERE cap.value = req.value
                    )
                )
            )
        "#);
        query.push_bind(capabilities_to_json(&params.capabilities));
        
        // Priority filter
        if let Some(min_priority) = params.min_priority {
            query.push(" AND t.priority_score >= ");
            query.push_bind(min_priority);
        }
        
        // Order by priority and age
        query.push(r#"
            ORDER BY 
                CASE t.state
                    WHEN 'Blocked' THEN 1
                    WHEN 'Review' THEN 2
                    WHEN 'PendingHandoff' THEN 3
                    WHEN 'InProgress' THEN 4
                    WHEN 'Created' THEN 5
                    ELSE 6
                END,
                t.priority_score DESC,
                t.inserted_at ASC
            LIMIT ?
        "#);
        query.push_bind(params.max_tasks);
        
        let tasks = query.build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        tasks.into_iter()
            .map(|row| self.row_to_task(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_team_workload(&self) -> Result<TeamWorkloadSummary> {
        let summary = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_agents,
                SUM(CASE WHEN status IN ('active', 'blocked') THEN 1 ELSE 0 END) as active_agents,
                SUM(max_concurrent_tasks) as total_capacity,
                SUM(current_load) as current_total_load,
                AVG(CAST(current_load AS REAL) / CAST(max_concurrent_tasks AS REAL) * 100) as avg_load_percentage
            FROM agents
            WHERE status != 'offline'
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let total_agents: i32 = summary.get("total_agents");
        let active_agents: i32 = summary.get("active_agents");
        let total_capacity: i32 = summary.get("total_capacity");
        let current_total_load: i32 = summary.get("current_total_load");
        let avg_load_percentage: f64 = summary.get("avg_load_percentage");
        
        // Get agents at capacity
        let at_capacity = sqlx::query_scalar::<_, String>(
            r#"
            SELECT name
            FROM agents
            WHERE current_load >= max_concurrent_tasks
              AND status IN ('active', 'blocked')
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Get idle agents
        let idle = sqlx::query_scalar::<_, String>(
            r#"
            SELECT name
            FROM agents
            WHERE status = 'idle'
              AND current_load = 0
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Get blocked agents
        let blocked = sqlx::query_scalar::<_, String>(
            r#"
            SELECT name
            FROM agents
            WHERE status = 'blocked'
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(TeamWorkloadSummary {
            total_agents,
            active_agents,
            total_capacity,
            current_total_load,
            agents_at_capacity: at_capacity,
            idle_agents: idle,
            blocked_agents: blocked,
            average_load_percentage: avg_load_percentage,
        })
    }
    
    async fn get_agent_statistics(&self, agent_name: &str) -> Result<AgentStatistics> {
        // Basic stats
        let stats = sqlx::query(
            r#"
            SELECT 
                a.name,
                a.total_tasks_completed,
                a.total_tasks_failed,
                a.success_rate,
                COUNT(DISTINCT CASE 
                    WHEN t.state = 'Done' AND t.done_at >= datetime('now', '-7 days') 
                    THEN t.code 
                END) as last_7_days,
                COUNT(DISTINCT CASE 
                    WHEN t.state = 'Done' AND t.done_at >= datetime('now', '-30 days') 
                    THEN t.code 
                END) as last_30_days
            FROM agents a
            LEFT JOIN tasks t ON t.owner_agent_name = a.name
            WHERE a.name = ?
            GROUP BY a.id
            "#
        )
        .bind(agent_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", agent_name)))?;
        
        // Average duration from work sessions
        let avg_duration = sqlx::query_scalar::<_, Option<f64>>(
            r#"
            SELECT AVG(total_active_minutes)
            FROM work_sessions
            WHERE agent_name = ?
              AND finished_at IS NOT NULL
            "#
        )
        .bind(agent_name)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .unwrap_or(0.0);
        
        // Current streak (consecutive successful tasks)
        // This is simplified - a proper implementation would track task order
        let current_streak = 0; // TODO: Implement proper streak tracking
        
        // Specialization scores (simplified - based on task count per capability)
        let specializations = HashMap::new(); // TODO: Implement specialization scoring
        
        Ok(AgentStatistics {
            agent_name: stats.get("name"),
            total_tasks_completed: stats.get("total_tasks_completed"),
            average_task_duration_minutes: avg_duration,
            success_rate: stats.get("success_rate"),
            current_streak,
            specialization_scores: specializations,
            last_7_days_tasks: stats.get("last_7_days"),
            last_30_days_tasks: stats.get("last_30_days"),
        })
    }
}
```

### 3. Add Helper Methods
```rust
impl SqliteTaskRepository {
    fn row_to_agent_profile(&self, row: SqliteRow) -> Result<AgentProfile> {
        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "idle" => AgentStatus::Idle,
            "active" => AgentStatus::Active,
            "blocked" => AgentStatus::Blocked,
            "unresponsive" => AgentStatus::Unresponsive,
            "offline" => AgentStatus::Offline,
            _ => AgentStatus::Idle,
        };
        
        let capabilities_json: String = row.get("capabilities");
        let capabilities = parse_capabilities(&capabilities_json)?;
        
        let specializations_json: String = row.get("specializations");
        let specializations = parse_capabilities(&specializations_json)?;
        
        let preferences_json: String = row.get("preferences");
        let preferences = serde_json::from_str(&preferences_json)
            .unwrap_or_else(|_| serde_json::json!({}));
        
        Ok(AgentProfile {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            capabilities,
            max_concurrent_tasks: row.get("max_concurrent_tasks"),
            current_load: row.get("current_load"),
            status,
            preferences,
            last_heartbeat: row.get("last_heartbeat"),
            reputation_score: row.get("reputation_score"),
            specializations,
            registered_at: row.get("registered_at"),
            registered_by: row.get("registered_by"),
        })
    }
}
```

### 4. Create Agent Helper Module
Create `database/src/agent_helpers.rs`:

```rust
use crate::error::{Result, TaskError};

/// Parse capabilities from JSON string
pub fn parse_capabilities(capabilities_json: &str) -> Result<Vec<String>> {
    serde_json::from_str(capabilities_json)
        .map_err(|e| TaskError::Database(format!("Invalid capabilities JSON: {}", e)))
}

/// Convert capabilities to JSON string
pub fn capabilities_to_json(capabilities: &[String]) -> String {
    serde_json::to_string(capabilities).unwrap_or_else(|_| "[]".to_string())
}

/// Check if agent has specific capability
pub fn has_capability(capabilities_json: &str, capability: &str) -> bool {
    parse_capabilities(capabilities_json)
        .map(|caps| caps.iter().any(|c| c == capability))
        .unwrap_or(false)
}

/// Calculate capability match percentage
pub fn calculate_capability_match(
    agent_capabilities: &[String],
    required_capabilities: &[String],
) -> f64 {
    if required_capabilities.is_empty() {
        return 1.0;
    }
    
    let matched = required_capabilities.iter()
        .filter(|req| agent_capabilities.contains(req))
        .count();
    
    matched as f64 / required_capabilities.len() as f64
}
```

## Files to Modify
- `database/src/sqlite.rs` - Add agent method implementations
- `database/src/agent_helpers.rs` - New file with helper functions
- `database/src/lib.rs` - Export agent helpers module

## Testing Requirements
1. Test agent registration with validation
2. Test capability matching algorithms
3. Test workload updates with constraints
4. Test heartbeat and unresponsive detection
5. Test work discovery with various filters
6. Test reputation updates with bounds
7. Test concurrent load updates

## Performance Considerations
1. Capability searches use JSON - consider normalized table for large scale
2. Work discovery query is complex - ensure proper indexes
3. Heartbeat updates should be batched if many agents
4. Consider caching agent profiles for frequently accessed agents

## Security Considerations
1. Validate all agent names are kebab-case
2. Prevent load manipulation beyond limits
3. Ensure reputation scores stay in bounds
4. Rate limit heartbeat updates to prevent spam