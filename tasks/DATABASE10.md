# DATABASE10: Implement Help Requests Repository

## Objective
Implement all help request methods in the SQLite repository, enabling agents to request and provide assistance through a structured help system.

## Implementation Details

### 1. Add Help Request Methods to SqliteTaskRepository
In `database/src/sqlite.rs`, add implementations for help request methods:

```rust
impl TaskRepository for SqliteTaskRepository {
    // ... existing implementations ...
    
    async fn create_help_request(&self, request: NewHelpRequest) -> Result<HelpRequest> {
        // Validate input
        request.validate()?;
        
        // Validate task exists
        let task_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE code = ?)"
        )
        .bind(&request.task_code)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if !task_exists {
            return Err(TaskError::NotFound(format!("Task {} not found", request.task_code)));
        }
        
        // Validate agent exists
        let agent_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE name = ?)"
        )
        .bind(&request.requesting_agent_name)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if !agent_exists {
            return Err(TaskError::NotFound(
                format!("Agent {} not found", request.requesting_agent_name)
            ));
        }
        
        // Convert related capabilities to JSON
        let capabilities_json = serde_json::to_string(&request.related_capabilities)
            .map_err(|e| TaskError::Serialization(
                format!("Failed to serialize capabilities: {}", e)
            ))?;
        
        // Insert help request
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO help_requests 
            (requesting_agent_name, task_code, help_type, description, 
             urgency, related_capabilities)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(&request.requesting_agent_name)
        .bind(&request.task_code)
        .bind(request.help_type.to_string())
        .bind(&request.description)
        .bind(request.urgency.to_string())
        .bind(&capabilities_json)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Log event
        let event = SystemEvent {
            id: 0, // Will be auto-generated
            timestamp: Utc::now(),
            event_type: "help_request_created".to_string(),
            actor_type: ActorType::Agent,
            actor_id: request.requesting_agent_name.clone(),
            task_code: Some(request.task_code.clone()),
            payload: serde_json::json!({
                "help_request_id": id,
                "help_type": request.help_type.to_string(),
                "urgency": request.urgency.to_string(),
            }),
            correlation_id: Some(format!("help-{}", id)),
        };
        
        self.log_event(event).await?;
        
        // Fetch and return created request
        self.get_help_request(id)
            .await?
            .ok_or_else(|| TaskError::Database("Failed to fetch created help request".to_string()))
    }
    
    async fn get_help_request(&self, request_id: i32) -> Result<Option<HelpRequest>> {
        let row = sqlx::query(
            r#"
            SELECT id, requesting_agent_name, task_code, help_type, description,
                   urgency, created_at, resolved_at, resolved_by, resolution,
                   claimed_by, claimed_at, related_capabilities
            FROM help_requests
            WHERE id = ?
            "#
        )
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        row.map(|r| self.row_to_help_request(r)).transpose()
    }
    
    async fn list_help_requests(&self, filter: HelpRequestFilter) -> Result<Vec<HelpRequest>> {
        let mut query = QueryBuilder::new(
            r#"
            SELECT id, requesting_agent_name, task_code, help_type, description,
                   urgency, created_at, resolved_at, resolved_by, resolution,
                   claimed_by, claimed_at, related_capabilities
            FROM help_requests
            WHERE 1=1
            "#
        );
        
        // Apply filters
        if !filter.help_types.is_empty() {
            query.push(" AND help_type IN (");
            let mut separated = query.separated(", ");
            for help_type in &filter.help_types {
                separated.push_bind(help_type.to_string());
            }
            query.push(")");
        }
        
        if let Some(min_urgency) = &filter.urgency_min {
            query.push(" AND ");
            query.push("CASE urgency ");
            query.push("WHEN 'critical' THEN 4 ");
            query.push("WHEN 'high' THEN 3 ");
            query.push("WHEN 'medium' THEN 2 ");
            query.push("WHEN 'low' THEN 1 ");
            query.push("END >= ");
            query.push_bind(min_urgency.priority());
        }
        
        if let Some(status) = &filter.status {
            match status {
                HelpRequestStatus::Open => {
                    query.push(" AND resolved_at IS NULL AND claimed_by IS NULL");
                }
                HelpRequestStatus::Claimed => {
                    query.push(" AND resolved_at IS NULL AND claimed_by IS NOT NULL");
                }
                HelpRequestStatus::Resolved => {
                    query.push(" AND resolved_at IS NOT NULL");
                }
            }
        }
        
        if let Some(task_code) = &filter.task_code {
            query.push(" AND task_code = ");
            query.push_bind(task_code);
        }
        
        if let Some(requesting_agent) = &filter.requesting_agent {
            query.push(" AND requesting_agent_name = ");
            query.push_bind(requesting_agent);
        }
        
        if let Some(claimed_by) = &filter.claimed_by {
            query.push(" AND claimed_by = ");
            query.push_bind(claimed_by);
        }
        
        // Filter by capabilities
        if !filter.capabilities.is_empty() {
            query.push(" AND (");
            let mut first = true;
            for capability in &filter.capabilities {
                if !first {
                    query.push(" OR ");
                }
                query.push(" EXISTS (");
                query.push("SELECT 1 FROM json_each(related_capabilities) WHERE value = ");
                query.push_bind(capability);
                query.push(")");
                first = false;
            }
            query.push(")");
        }
        
        if let Some(since) = filter.since {
            query.push(" AND created_at >= ");
            query.push_bind(since);
        }
        
        // Order by urgency and age
        query.push(" ORDER BY ");
        query.push("CASE urgency ");
        query.push("WHEN 'critical' THEN 4 ");
        query.push("WHEN 'high' THEN 3 ");
        query.push("WHEN 'medium' THEN 2 ");
        query.push("WHEN 'low' THEN 1 ");
        query.push("END DESC, created_at ASC");
        
        // Apply pagination
        if let Some(limit) = filter.limit {
            query.push(" LIMIT ");
            query.push_bind(limit);
        }
        
        if let Some(offset) = filter.offset {
            query.push(" OFFSET ");
            query.push_bind(offset);
        }
        
        let requests = query.build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        requests.into_iter()
            .map(|row| self.row_to_help_request(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn resolve_help_request(&self, resolution: HelpRequestResolution) -> Result<()> {
        resolution.validate()?;
        
        // Start transaction
        let mut tx = self.pool.begin().await.map_err(sqlx_error_to_task_error)?;
        
        // Get request details
        let request = self.get_help_request(resolution.request_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Help request {} not found", resolution.request_id)
            ))?;
        
        if request.resolved_at.is_some() {
            return Err(TaskError::Validation("Help request already resolved".to_string()));
        }
        
        // Update help request
        sqlx::query(
            r#"
            UPDATE help_requests 
            SET resolved_at = CURRENT_TIMESTAMP,
                resolved_by = ?,
                resolution = ?
            WHERE id = ?
            "#
        )
        .bind(&resolution.resolved_by)
        .bind(&resolution.resolution)
        .bind(resolution.request_id)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Log resolution event
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "help_request_resolved".to_string(),
            actor_type: ActorType::Agent,
            actor_id: resolution.resolved_by.clone(),
            task_code: Some(request.task_code.clone()),
            payload: serde_json::json!({
                "help_request_id": resolution.request_id,
                "resolution_time_minutes": request.age_minutes(),
            }),
            correlation_id: Some(format!("help-{}", resolution.request_id)),
        };
        
        self.log_event(event).await?;
        
        // If it was a blocker, check if task can be unblocked
        if request.help_type == HelpType::Blocker {
            sqlx::query(
                r#"
                UPDATE tasks 
                SET state = 'InProgress'
                WHERE code = ? 
                  AND state = 'Blocked'
                  AND NOT EXISTS (
                      SELECT 1 FROM help_requests
                      WHERE task_code = tasks.code
                        AND help_type = 'blocker'
                        AND resolved_at IS NULL
                  )
                "#
            )
            .bind(&request.task_code)
            .execute(&mut *tx)
            .await
            .map_err(sqlx_error_to_task_error)?;
        }
        
        tx.commit().await.map_err(sqlx_error_to_task_error)?;
        
        Ok(())
    }
    
    async fn claim_help_request(&self, request_id: i32, agent_name: &str) -> Result<()> {
        // Verify agent exists
        let agent_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE name = ?)"
        )
        .bind(agent_name)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if !agent_exists {
            return Err(TaskError::NotFound(format!("Agent {} not found", agent_name)));
        }
        
        // Get request to verify it can be claimed
        let request = self.get_help_request(request_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Help request {} not found", request_id)
            ))?;
        
        if request.resolved_at.is_some() {
            return Err(TaskError::Validation("Help request already resolved".to_string()));
        }
        
        if request.claimed_by.is_some() {
            return Err(TaskError::Validation("Help request already claimed".to_string()));
        }
        
        // Verify agent has required capabilities if specified
        if !request.related_capabilities.is_empty() {
            let agent = self.get_agent(agent_name)
                .await?
                .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", agent_name)))?;
            
            let has_capability = request.related_capabilities.iter()
                .any(|cap| agent.has_capability(cap));
            
            if !has_capability {
                return Err(TaskError::Validation(
                    "Agent does not have required capabilities".to_string()
                ));
            }
        }
        
        // Update claim
        sqlx::query(
            r#"
            UPDATE help_requests 
            SET claimed_by = ?, claimed_at = CURRENT_TIMESTAMP
            WHERE id = ? AND claimed_by IS NULL
            "#
        )
        .bind(agent_name)
        .bind(request_id)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Log claim event
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "help_request_claimed".to_string(),
            actor_type: ActorType::Agent,
            actor_id: agent_name.to_string(),
            task_code: Some(request.task_code),
            payload: serde_json::json!({
                "help_request_id": request_id,
                "wait_time_minutes": request.age_minutes(),
            }),
            correlation_id: Some(format!("help-{}", request_id)),
        };
        
        self.log_event(event).await?;
        
        Ok(())
    }
    
    async fn escalate_help_request(&self, request_id: i32, new_urgency: Urgency) -> Result<()> {
        // Get current request
        let request = self.get_help_request(request_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Help request {} not found", request_id)
            ))?;
        
        if request.resolved_at.is_some() {
            return Err(TaskError::Validation("Cannot escalate resolved request".to_string()));
        }
        
        if request.urgency.priority() >= new_urgency.priority() {
            return Err(TaskError::Validation(
                "New urgency must be higher than current".to_string()
            ));
        }
        
        // Update urgency
        sqlx::query(
            "UPDATE help_requests SET urgency = ? WHERE id = ?"
        )
        .bind(new_urgency.to_string())
        .bind(request_id)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Log escalation event
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "help_request_escalated".to_string(),
            actor_type: ActorType::System,
            actor_id: "escalation-system".to_string(),
            task_code: Some(request.task_code),
            payload: serde_json::json!({
                "help_request_id": request_id,
                "old_urgency": request.urgency.to_string(),
                "new_urgency": new_urgency.to_string(),
                "age_minutes": request.age_minutes(),
            }),
            correlation_id: Some(format!("help-{}", request_id)),
        };
        
        self.log_event(event).await?;
        
        Ok(())
    }
    
    async fn get_agent_help_requests(
        &self, 
        agent_name: &str, 
        include_resolved: bool
    ) -> Result<Vec<HelpRequest>> {
        let mut query = String::from(
            r#"
            SELECT id, requesting_agent_name, task_code, help_type, description,
                   urgency, created_at, resolved_at, resolved_by, resolution,
                   claimed_by, claimed_at, related_capabilities
            FROM help_requests
            WHERE requesting_agent_name = ?
            "#
        );
        
        if !include_resolved {
            query.push_str(" AND resolved_at IS NULL");
        }
        
        query.push_str(" ORDER BY created_at DESC");
        
        let requests = sqlx::query(&query)
            .bind(agent_name)
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        requests.into_iter()
            .map(|row| self.row_to_help_request(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_help_by_capability(&self, capability: &str) -> Result<Vec<HelpRequest>> {
        let requests = sqlx::query(
            r#"
            SELECT id, requesting_agent_name, task_code, help_type, description,
                   urgency, created_at, resolved_at, resolved_by, resolution,
                   claimed_by, claimed_at, related_capabilities
            FROM help_requests
            WHERE resolved_at IS NULL
              AND claimed_by IS NULL
              AND EXISTS (
                  SELECT 1 FROM json_each(related_capabilities)
                  WHERE value = ?
              )
            ORDER BY 
                CASE urgency
                    WHEN 'critical' THEN 4
                    WHEN 'high' THEN 3
                    WHEN 'medium' THEN 2
                    WHEN 'low' THEN 1
                END DESC,
                created_at ASC
            "#
        )
        .bind(capability)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        requests.into_iter()
            .map(|row| self.row_to_help_request(row))
            .collect::<Result<Vec<_>>>()
    }
}
```

### 2. Add Helper Methods
```rust
impl SqliteTaskRepository {
    fn row_to_help_request(&self, row: SqliteRow) -> Result<HelpRequest> {
        let help_type_str: String = row.get("help_type");
        let help_type = match help_type_str.as_str() {
            "technical_question" => HelpType::TechnicalQuestion,
            "blocker" => HelpType::Blocker,
            "review" => HelpType::Review,
            "clarification" => HelpType::Clarification,
            "escalation" => HelpType::Escalation,
            _ => return Err(TaskError::Database(
                format!("Unknown help type: {}", help_type_str)
            )),
        };
        
        let urgency_str: String = row.get("urgency");
        let urgency = match urgency_str.as_str() {
            "low" => Urgency::Low,
            "medium" => Urgency::Medium,
            "high" => Urgency::High,
            "critical" => Urgency::Critical,
            _ => return Err(TaskError::Database(
                format!("Unknown urgency: {}", urgency_str)
            )),
        };
        
        let capabilities_json: String = row.get("related_capabilities");
        let related_capabilities = serde_json::from_str::<Vec<String>>(&capabilities_json)
            .unwrap_or_default();
        
        Ok(HelpRequest {
            id: row.get("id"),
            requesting_agent_name: row.get("requesting_agent_name"),
            task_code: row.get("task_code"),
            help_type,
            description: row.get("description"),
            urgency,
            created_at: row.get("created_at"),
            resolved_at: row.get("resolved_at"),
            resolved_by: row.get("resolved_by"),
            resolution: row.get("resolution"),
            claimed_by: row.get("claimed_by"),
            claimed_at: row.get("claimed_at"),
            related_capabilities,
        })
    }
}
```

### 3. Add Auto-Escalation Background Task
Create `database/src/help_escalation.rs`:

```rust
use crate::{TaskRepository, Urgency};
use chrono::{Utc, Duration};
use tokio::time::sleep;
use std::time::Duration as StdDuration;

/// Background task to auto-escalate old help requests
pub async fn auto_escalate_help_requests<R: TaskRepository>(
    repository: R,
    check_interval_seconds: u64,
) {
    loop {
        // Sleep first
        sleep(StdDuration::from_secs(check_interval_seconds)).await;
        
        // Get unresolved help requests
        let filter = HelpRequestFilter {
            status: Some(HelpRequestStatus::Open),
            ..Default::default()
        };
        
        if let Ok(requests) = repository.list_help_requests(filter).await {
            for request in requests {
                // Skip if already at max urgency
                if request.urgency == Urgency::Critical {
                    continue;
                }
                
                // Check age and escalate if needed
                let age_minutes = request.age_minutes();
                let threshold = request.help_type
                    .suggested_response_minutes(request.urgency) * 2;
                
                if age_minutes > threshold as i64 {
                    let new_urgency = match request.urgency {
                        Urgency::Low => Urgency::Medium,
                        Urgency::Medium => Urgency::High,
                        Urgency::High => Urgency::Critical,
                        Urgency::Critical => Urgency::Critical,
                    };
                    
                    if let Err(e) = repository.escalate_help_request(
                        request.id, 
                        new_urgency
                    ).await {
                        tracing::error!(
                            "Failed to auto-escalate help request {}: {}", 
                            request.id, 
                            e
                        );
                    } else {
                        tracing::info!(
                            "Auto-escalated help request {} from {:?} to {:?}", 
                            request.id,
                            request.urgency,
                            new_urgency
                        );
                    }
                }
            }
        }
    }
}
```

### 4. Add Help Request Notification System
Create `database/src/help_notifications.rs`:

```rust
use crate::{TaskRepository, HelpRequest, AgentProfile};

/// Find agents who can help with a request
pub async fn find_helpers<R: TaskRepository>(
    repository: &R,
    request: &HelpRequest,
) -> Result<Vec<AgentProfile>> {
    let mut suitable_agents = Vec::new();
    
    // If capabilities specified, find agents with those capabilities
    if !request.related_capabilities.is_empty() {
        for capability in &request.related_capabilities {
            let matches = repository
                .find_agents_by_capability(capability, 10)
                .await?;
            
            for agent_match in matches {
                let agent = repository
                    .get_agent(&agent_match.agent_name)
                    .await?
                    .ok_or_else(|| TaskError::NotFound(
                        format!("Agent {} not found", agent_match.agent_name)
                    ))?;
                
                if agent.is_available() && 
                   agent.name != request.requesting_agent_name {
                    suitable_agents.push(agent);
                }
            }
        }
    } else {
        // Get any available agents
        let filter = AgentFilter::available_only();
        let agents = repository.list_agents(filter).await?;
        
        suitable_agents = agents.into_iter()
            .filter(|a| a.name != request.requesting_agent_name)
            .collect();
    }
    
    // Sort by reputation and current load
    suitable_agents.sort_by(|a, b| {
        b.reputation_score.partial_cmp(&a.reputation_score)
            .unwrap()
            .then_with(|| a.current_load.cmp(&b.current_load))
    });
    
    Ok(suitable_agents)
}

/// Create notification events for help request
pub async fn notify_agents<R: TaskRepository>(
    repository: &R,
    request: &HelpRequest,
    target_agents: Vec<String>,
) -> Result<()> {
    for agent_name in target_agents {
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "help_request_notification".to_string(),
            actor_type: ActorType::System,
            actor_id: "notification-system".to_string(),
            task_code: Some(request.task_code.clone()),
            payload: serde_json::json!({
                "help_request_id": request.id,
                "help_type": request.help_type.to_string(),
                "urgency": request.urgency.to_string(),
                "target_agent": agent_name,
                "requesting_agent": request.requesting_agent_name,
            }),
            correlation_id: Some(format!("help-{}", request.id)),
        };
        
        repository.log_event(event).await?;
    }
    
    Ok(())
}
```

## Files to Modify
- `database/src/sqlite.rs` - Add help request method implementations
- `database/src/help_escalation.rs` - New file for auto-escalation
- `database/src/help_notifications.rs` - New file for notification system
- `database/src/lib.rs` - Export help modules

## Testing Requirements
1. Test help request creation with validation
2. Test filtering by various criteria
3. Test claim and resolution flow
4. Test escalation logic
5. Test capability-based routing
6. Test auto-escalation background task
7. Test notification system

## Performance Considerations
1. Urgency-based ordering in queries
2. JSON capability searches may need optimization
3. Consider caching open help requests
4. Background escalation should be efficient
5. Notification batching for many agents

## Security Considerations
1. Validate agent permissions for claims
2. Prevent self-resolution of own requests
3. Ensure capability requirements are enforced
4. Rate limit help request creation
5. Audit trail via system events