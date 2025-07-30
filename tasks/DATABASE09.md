# DATABASE09: Implement Analytics Repository

## Objective
Implement all analytics and metrics methods in the SQLite repository, including system events, performance metrics, and comprehensive reporting capabilities.

## Implementation Details

### 1. Add System Event Methods to SqliteTaskRepository
In `database/src/sqlite.rs`, add implementations for event-related methods:

```rust
impl TaskRepository for SqliteTaskRepository {
    // ... existing implementations ...
    
    async fn log_event(&self, event: SystemEvent) -> Result<()> {
        let payload_json = serde_json::to_string(&event.payload)
            .map_err(|e| TaskError::Serialization(format!("Failed to serialize payload: {}", e)))?;
        
        sqlx::query(
            r#"
            INSERT INTO system_events 
            (event_type, actor_type, actor_id, task_code, payload, correlation_id)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&event.event_type)
        .bind(event.actor_type.to_string())
        .bind(&event.actor_id)
        .bind(&event.task_code)
        .bind(&payload_json)
        .bind(&event.correlation_id)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(())
    }
    
    async fn get_events(&self, filter: EventFilter) -> Result<Vec<SystemEvent>> {
        let mut query = QueryBuilder::new(
            r#"
            SELECT id, timestamp, event_type, actor_type, actor_id, 
                   task_code, payload, correlation_id
            FROM system_events
            WHERE timestamp BETWEEN 
            "#
        );
        
        // Time range filter
        query.push_bind(filter.time_range.start);
        query.push(" AND ");
        query.push_bind(filter.time_range.end);
        
        // Event type filter
        if !filter.event_types.is_empty() {
            query.push(" AND event_type IN (");
            let mut separated = query.separated(", ");
            for event_type in &filter.event_types {
                separated.push_bind(event_type);
            }
            query.push(")");
        }
        
        // Actor type filter
        if let Some(actor_type) = &filter.actor_type {
            query.push(" AND actor_type = ");
            query.push_bind(actor_type.to_string());
        }
        
        // Actor ID filter
        if let Some(actor_id) = &filter.actor_id {
            query.push(" AND actor_id = ");
            query.push_bind(actor_id);
        }
        
        // Task code filter
        if let Some(task_code) = &filter.task_code {
            query.push(" AND task_code = ");
            query.push_bind(task_code);
        }
        
        // Order by timestamp descending
        query.push(" ORDER BY timestamp DESC");
        
        // Pagination
        if let Some(limit) = filter.limit {
            query.push(" LIMIT ");
            query.push_bind(limit);
        }
        
        if let Some(offset) = filter.offset {
            query.push(" OFFSET ");
            query.push_bind(offset);
        }
        
        let events = query.build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        events.into_iter()
            .map(|row| self.row_to_system_event(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_task_events(&self, task_code: &str, limit: i32) -> Result<Vec<SystemEvent>> {
        let events = sqlx::query(
            r#"
            SELECT id, timestamp, event_type, actor_type, actor_id, 
                   task_code, payload, correlation_id
            FROM system_events
            WHERE task_code = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#
        )
        .bind(task_code)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        events.into_iter()
            .map(|row| self.row_to_system_event(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_correlated_events(&self, correlation_id: &str) -> Result<Vec<SystemEvent>> {
        let events = sqlx::query(
            r#"
            SELECT id, timestamp, event_type, actor_type, actor_id, 
                   task_code, payload, correlation_id
            FROM system_events
            WHERE correlation_id = ?
            ORDER BY timestamp
            "#
        )
        .bind(correlation_id)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        events.into_iter()
            .map(|row| self.row_to_system_event(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_event_stats(&self, time_range: TimeRange) -> Result<EventStatistics> {
        // Get total events
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM system_events WHERE timestamp BETWEEN ? AND ?"
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Get counts by type
        let by_type_rows = sqlx::query(
            r#"
            SELECT event_type, COUNT(*) as count
            FROM system_events
            WHERE timestamp BETWEEN ? AND ?
            GROUP BY event_type
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut by_type = HashMap::new();
        for row in by_type_rows {
            let event_type: String = row.get("event_type");
            let count: i64 = row.get("count");
            by_type.insert(event_type, count);
        }
        
        // Get counts by actor type
        let by_actor_rows = sqlx::query(
            r#"
            SELECT actor_type, COUNT(*) as count
            FROM system_events
            WHERE timestamp BETWEEN ? AND ?
            GROUP BY actor_type
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut by_actor_type = HashMap::new();
        for row in by_actor_rows {
            let actor_type_str: String = row.get("actor_type");
            let count: i64 = row.get("count");
            if let Ok(actor_type) = ActorType::try_from(actor_type_str.as_str()) {
                by_actor_type.insert(actor_type, count);
            }
        }
        
        // Calculate events per hour
        let duration_hours = (time_range.end - time_range.start).num_hours() as f64;
        let events_per_hour = if duration_hours > 0.0 {
            total as f64 / duration_hours
        } else {
            0.0
        };
        
        // Find peak hour
        let peak_hour = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            r#"
            SELECT 
                datetime(strftime('%Y-%m-%d %H:00:00', timestamp)) as hour
            FROM system_events
            WHERE timestamp BETWEEN ? AND ?
            GROUP BY hour
            ORDER BY COUNT(*) DESC
            LIMIT 1
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(EventStatistics {
            total_events: total,
            by_type,
            by_actor_type,
            events_per_hour,
            peak_hour,
        })
    }
}
```

### 2. Add Analytics Methods Implementation
```rust
impl TaskRepository for SqliteTaskRepository {
    async fn get_task_metrics(&self, time_range: TimeRange) -> Result<TaskMetrics> {
        // Get task counts by state
        let state_counts = sqlx::query(
            r#"
            SELECT state, COUNT(*) as count
            FROM tasks
            WHERE inserted_at BETWEEN ? AND ?
            GROUP BY state
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut by_state = HashMap::new();
        let mut total_created = 0;
        let mut total_completed = 0;
        let mut total_failed = 0;
        
        for row in state_counts {
            let state_str: String = row.get("state");
            let count: i32 = row.get("count");
            
            if let Ok(state) = TaskState::try_from(state_str.as_str()) {
                by_state.insert(state.clone(), count);
                
                total_created += count;
                if state == TaskState::Done {
                    total_completed = count;
                } else if state == TaskState::Archived {
                    total_failed += count;
                }
            }
        }
        
        // Get priority distribution
        let priority_counts = sqlx::query(
            r#"
            SELECT priority_score, COUNT(*) as count
            FROM tasks
            WHERE inserted_at BETWEEN ? AND ?
            GROUP BY priority_score
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut by_priority = HashMap::new();
        for row in priority_counts {
            let priority: i32 = row.get("priority_score");
            let count: i32 = row.get("count");
            by_priority.insert(priority, count);
        }
        
        // Calculate completion rate
        let completion_rate = if total_created > 0 {
            (total_completed as f64 / total_created as f64) * 100.0
        } else {
            0.0
        };
        
        // Get average duration
        let avg_duration = sqlx::query_scalar::<_, Option<f64>>(
            r#"
            SELECT AVG(
                (julianday(done_at) - julianday(inserted_at)) * 24 * 60
            ) as avg_minutes
            FROM tasks
            WHERE state = 'Done'
              AND done_at IS NOT NULL
              AND done_at BETWEEN ? AND ?
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .unwrap_or(0.0);
        
        // Count overdue tasks
        let overdue_count = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COUNT(*)
            FROM tasks
            WHERE state NOT IN ('Done', 'Archived')
              AND datetime('now') > datetime(inserted_at, '+7 days')
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(TaskMetrics {
            total_created,
            total_completed,
            total_failed,
            completion_rate,
            average_duration_minutes: avg_duration,
            by_state,
            by_priority,
            overdue_count,
        })
    }
    
    async fn get_agent_metrics(&self, agent_name: &str, time_range: TimeRange) -> Result<AgentMetrics> {
        // Get agent basic info
        let agent = self.get_agent(agent_name)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", agent_name)))?;
        
        // Get task completion stats
        let task_stats = sqlx::query(
            r#"
            SELECT 
                COUNT(CASE WHEN state = 'Done' THEN 1 END) as completed,
                COUNT(CASE WHEN state = 'Archived' AND failure_count > 0 THEN 1 END) as failed,
                AVG(CASE 
                    WHEN state = 'Done' AND done_at IS NOT NULL 
                    THEN (julianday(done_at) - julianday(inserted_at)) * 24 * 60 
                END) as avg_duration
            FROM tasks
            WHERE owner_agent_name = ?
              AND inserted_at BETWEEN ? AND ?
            "#
        )
        .bind(agent_name)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let tasks_completed: i32 = task_stats.get("completed");
        let tasks_failed: i32 = task_stats.get("failed");
        let avg_duration: Option<f64> = task_stats.get("avg_duration");
        
        // Calculate success rate
        let total_tasks = tasks_completed + tasks_failed;
        let success_rate = if total_tasks > 0 {
            (tasks_completed as f64 / total_tasks as f64) * 100.0
        } else {
            100.0
        };
        
        // Get active time from work sessions
        let active_minutes = sqlx::query_scalar::<_, Option<i32>>(
            r#"
            SELECT SUM(total_active_minutes)
            FROM work_sessions
            WHERE agent_name = ?
              AND started_at BETWEEN ? AND ?
            "#
        )
        .bind(agent_name)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .unwrap_or(0);
        
        // Calculate idle time percentage
        let total_minutes = (time_range.end - time_range.start).num_minutes() as f64;
        let idle_percentage = if total_minutes > 0.0 {
            ((total_minutes - active_minutes as f64) / total_minutes) * 100.0
        } else {
            0.0
        };
        
        // Get handoff counts
        let handoffs_created = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COUNT(*)
            FROM handoffs
            WHERE from_agent_name = ?
              AND created_at BETWEEN ? AND ?
            "#
        )
        .bind(agent_name)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let handoffs_received = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COUNT(*)
            FROM handoffs
            WHERE accepted_by = ?
              AND accepted_at BETWEEN ? AND ?
            "#
        )
        .bind(agent_name)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Get message count
        let messages_sent = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COUNT(*)
            FROM task_messages
            WHERE author_agent_name = ?
              AND created_at BETWEEN ? AND ?
            "#
        )
        .bind(agent_name)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Get knowledge objects created
        let knowledge_created = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COUNT(*)
            FROM knowledge_objects
            WHERE author_agent_name = ?
              AND created_at BETWEEN ? AND ?
              AND is_archived = 0
            "#
        )
        .bind(agent_name)
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(AgentMetrics {
            agent_name: agent_name.to_string(),
            tasks_completed,
            tasks_failed,
            average_duration_minutes: avg_duration.unwrap_or(0.0),
            success_rate,
            current_load: agent.current_load,
            total_active_minutes: active_minutes,
            idle_time_percentage: idle_percentage,
            handoffs_created,
            handoffs_received,
            messages_sent,
            knowledge_objects_created: knowledge_created,
        })
    }
    
    async fn get_system_metrics(&self) -> Result<SystemMetrics> {
        // Get agent counts
        let agent_stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total,
                COUNT(CASE WHEN status IN ('active', 'blocked') THEN 1 END) as active
            FROM agents
            WHERE status != 'offline'
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let total_agents: i32 = agent_stats.get("total");
        let active_agents: i32 = agent_stats.get("active");
        
        // Get task counts
        let task_stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total,
                COUNT(CASE WHEN state IN ('InProgress', 'Review', 'Blocked') THEN 1 END) as active
            FROM tasks
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let total_tasks: i32 = task_stats.get("total");
        let active_tasks: i32 = task_stats.get("active");
        
        // Calculate tasks per hour (last 24 hours)
        let tasks_per_hour = sqlx::query_scalar::<_, f64>(
            r#"
            SELECT CAST(COUNT(*) AS REAL) / 24.0
            FROM tasks
            WHERE inserted_at >= datetime('now', '-24 hours')
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Average queue time
        let avg_queue_time = sqlx::query_scalar::<_, Option<f64>>(
            r#"
            SELECT AVG(
                CASE 
                    WHEN state = 'Created' 
                    THEN (julianday('now') - julianday(inserted_at)) * 24 * 60
                    ELSE (julianday(
                        COALESCE(
                            (SELECT MIN(timestamp) 
                             FROM system_events 
                             WHERE task_code = tasks.code 
                               AND event_type = 'task_started'),
                            inserted_at
                        )
                    ) - julianday(inserted_at)) * 24 * 60
                END
            )
            FROM tasks
            WHERE inserted_at >= datetime('now', '-7 days')
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .unwrap_or(0.0);
        
        // System load percentage
        let load_stats = sqlx::query(
            r#"
            SELECT 
                SUM(current_load) as total_load,
                SUM(max_concurrent_tasks) as total_capacity
            FROM agents
            WHERE status IN ('idle', 'active')
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let total_load: Option<i32> = load_stats.get("total_load");
        let total_capacity: Option<i32> = load_stats.get("total_capacity");
        
        let system_load_percentage = match (total_load, total_capacity) {
            (Some(load), Some(capacity)) if capacity > 0 => {
                (load as f64 / capacity as f64) * 100.0
            }
            _ => 0.0,
        };
        
        // Total counts
        let total_messages = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM task_messages"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let total_knowledge = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM knowledge_objects WHERE is_archived = 0"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Database size (simplified - would need platform-specific query)
        let database_size_mb = 0.0; // TODO: Implement actual size query
        
        Ok(SystemMetrics {
            total_agents,
            active_agents,
            total_tasks,
            active_tasks,
            tasks_per_hour,
            average_queue_time_minutes: avg_queue_time,
            system_load_percentage,
            total_messages,
            total_knowledge_objects: total_knowledge,
            database_size_mb,
        })
    }
    
    async fn get_duration_stats(&self, time_range: TimeRange) -> Result<DurationStatistics> {
        let mut by_state = HashMap::new();
        
        // Get duration stats for each state
        let states = vec![
            TaskState::Created,
            TaskState::InProgress,
            TaskState::Review,
            TaskState::Done,
        ];
        
        for state in states {
            let stats = sqlx::query(
                r#"
                SELECT 
                    MIN(duration) as min_duration,
                    MAX(duration) as max_duration,
                    AVG(duration) as avg_duration,
                    COUNT(*) as count
                FROM (
                    SELECT 
                        CASE 
                            WHEN ? = 'Done' AND done_at IS NOT NULL
                            THEN (julianday(done_at) - julianday(inserted_at)) * 24 * 60
                            ELSE (julianday('now') - julianday(inserted_at)) * 24 * 60
                        END as duration
                    FROM tasks
                    WHERE state = ?
                      AND inserted_at BETWEEN ? AND ?
                )
                WHERE duration IS NOT NULL
                "#
            )
            .bind(state.to_string())
            .bind(state.to_string())
            .bind(time_range.start)
            .bind(time_range.end)
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
            
            let min: Option<f64> = stats.get("min_duration");
            let max: Option<f64> = stats.get("max_duration");
            let avg: Option<f64> = stats.get("avg_duration");
            let count: i32 = stats.get("count");
            
            if count > 0 {
                // For median and percentiles, we'd need a more complex query
                // This is simplified
                by_state.insert(state, DurationStats {
                    min_minutes: min.unwrap_or(0.0),
                    max_minutes: max.unwrap_or(0.0),
                    avg_minutes: avg.unwrap_or(0.0),
                    median_minutes: avg.unwrap_or(0.0), // Simplified
                    p95_minutes: max.unwrap_or(0.0) * 0.95, // Simplified
                    p99_minutes: max.unwrap_or(0.0) * 0.99, // Simplified
                    sample_count: count,
                });
            }
        }
        
        // Calculate overall stats
        let overall_stats = sqlx::query(
            r#"
            SELECT 
                MIN(duration) as min_duration,
                MAX(duration) as max_duration,
                AVG(duration) as avg_duration,
                COUNT(*) as count
            FROM (
                SELECT 
                    CASE 
                        WHEN done_at IS NOT NULL
                        THEN (julianday(done_at) - julianday(inserted_at)) * 24 * 60
                        ELSE (julianday('now') - julianday(inserted_at)) * 24 * 60
                    END as duration
                FROM tasks
                WHERE inserted_at BETWEEN ? AND ?
            )
            WHERE duration IS NOT NULL
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let overall = DurationStats {
            min_minutes: overall_stats.get::<Option<f64>, _>("min_duration").unwrap_or(0.0),
            max_minutes: overall_stats.get::<Option<f64>, _>("max_duration").unwrap_or(0.0),
            avg_minutes: overall_stats.get::<Option<f64>, _>("avg_duration").unwrap_or(0.0),
            median_minutes: overall_stats.get::<Option<f64>, _>("avg_duration").unwrap_or(0.0),
            p95_minutes: overall_stats.get::<Option<f64>, _>("max_duration").unwrap_or(0.0) * 0.95,
            p99_minutes: overall_stats.get::<Option<f64>, _>("max_duration").unwrap_or(0.0) * 0.99,
            sample_count: overall_stats.get("count"),
        };
        
        Ok(DurationStatistics {
            by_state,
            overall,
        })
    }
    
    async fn get_help_request_stats(&self, time_range: TimeRange) -> Result<HelpRequestStatistics> {
        // Get basic counts
        let basic_stats = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total,
                COUNT(CASE WHEN resolved_at IS NOT NULL THEN 1 END) as resolved,
                AVG(CASE 
                    WHEN resolved_at IS NOT NULL 
                    THEN (julianday(resolved_at) - julianday(created_at)) * 24 * 60 
                END) as avg_resolution_time
            FROM help_requests
            WHERE created_at BETWEEN ? AND ?
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let total_requests: i32 = basic_stats.get("total");
        let resolved_requests: i32 = basic_stats.get("resolved");
        let avg_resolution_time: Option<f64> = basic_stats.get("avg_resolution_time");
        
        let resolution_rate = if total_requests > 0 {
            (resolved_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        // Get counts by type
        let type_counts = sqlx::query(
            r#"
            SELECT help_type, COUNT(*) as count
            FROM help_requests
            WHERE created_at BETWEEN ? AND ?
            GROUP BY help_type
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut by_type = HashMap::new();
        for row in type_counts {
            let help_type: String = row.get("help_type");
            let count: i32 = row.get("count");
            by_type.insert(help_type, count);
        }
        
        // Get counts by urgency
        let urgency_counts = sqlx::query(
            r#"
            SELECT urgency, COUNT(*) as count
            FROM help_requests
            WHERE created_at BETWEEN ? AND ?
            GROUP BY urgency
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut by_urgency = HashMap::new();
        for row in urgency_counts {
            let urgency: String = row.get("urgency");
            let count: i32 = row.get("count");
            by_urgency.insert(urgency, count);
        }
        
        // Get top requesters
        let top_requesters = sqlx::query(
            r#"
            SELECT requesting_agent_name, COUNT(*) as count
            FROM help_requests
            WHERE created_at BETWEEN ? AND ?
            GROUP BY requesting_agent_name
            ORDER BY count DESC
            LIMIT 5
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let top_requesters: Vec<(String, i32)> = top_requesters.into_iter()
            .map(|row| (row.get("requesting_agent_name"), row.get("count")))
            .collect();
        
        // Get top resolvers
        let top_resolvers = sqlx::query(
            r#"
            SELECT resolved_by, COUNT(*) as count
            FROM help_requests
            WHERE resolved_at IS NOT NULL
              AND created_at BETWEEN ? AND ?
            GROUP BY resolved_by
            ORDER BY count DESC
            LIMIT 5
            "#
        )
        .bind(time_range.start)
        .bind(time_range.end)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let top_resolvers: Vec<(String, i32)> = top_resolvers.into_iter()
            .map(|row| (row.get("resolved_by"), row.get("count")))
            .collect();
        
        Ok(HelpRequestStatistics {
            total_requests,
            resolved_requests,
            resolution_rate,
            average_resolution_time_minutes: avg_resolution_time.unwrap_or(0.0),
            by_type,
            by_urgency,
            top_requesters,
            top_resolvers,
        })
    }
    
    async fn get_workflow_metrics(&self, workflow_id: Option<i32>) -> Result<WorkflowMetrics> {
        let mut query = QueryBuilder::new(
            r#"
            SELECT 
                w.id as workflow_id,
                COUNT(DISTINCT t.code) as total_executions,
                COUNT(DISTINCT CASE WHEN t.state = 'Done' THEN t.code END) as completed_executions
            FROM workflows w
            LEFT JOIN tasks t ON t.workflow_definition_id = w.id
            WHERE 1=1
            "#
        );
        
        if let Some(id) = workflow_id {
            query.push(" AND w.id = ");
            query.push_bind(id);
        }
        
        query.push(" GROUP BY w.id");
        
        let workflow_stats = query.build()
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        let total_executions: i32 = workflow_stats.get("total_executions");
        let completed_executions: i32 = workflow_stats.get("completed_executions");
        
        let completion_rate = if total_executions > 0 {
            (completed_executions as f64 / total_executions as f64) * 100.0
        } else {
            0.0
        };
        
        // Get average duration from completed workflow steps
        let avg_duration = sqlx::query_scalar::<_, Option<f64>>(
            r#"
            SELECT AVG(total_duration)
            FROM (
                SELECT task_code, SUM(duration_minutes) as total_duration
                FROM completed_workflow_steps
                WHERE task_code IN (
                    SELECT code FROM tasks 
                    WHERE workflow_definition_id = COALESCE(?, workflow_definition_id)
                )
                GROUP BY task_code
            )
            "#
        )
        .bind(workflow_id)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .unwrap_or(0.0);
        
        // Get step metrics
        let step_metrics_rows = sqlx::query(
            r#"
            SELECT 
                step_id,
                COUNT(*) as execution_count,
                AVG(duration_minutes) as avg_duration,
                AVG(confidence_score) as avg_confidence,
                COUNT(CASE WHEN confidence_score >= 0.7 THEN 1 END) as success_count
            FROM completed_workflow_steps
            WHERE task_code IN (
                SELECT code FROM tasks 
                WHERE workflow_definition_id = COALESCE(?, workflow_definition_id)
            )
            GROUP BY step_id
            "#
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let step_metrics: Vec<StepMetric> = step_metrics_rows.into_iter()
            .map(|row| {
                let execution_count: i32 = row.get("execution_count");
                let success_count: i32 = row.get("success_count");
                
                StepMetric {
                    step_id: row.get("step_id"),
                    step_name: row.get::<String, _>("step_id"), // Would need join with workflow def
                    execution_count,
                    average_duration_minutes: row.get("avg_duration"),
                    success_rate: if execution_count > 0 {
                        (success_count as f64 / execution_count as f64) * 100.0
                    } else {
                        0.0
                    },
                    average_confidence: row.get("avg_confidence"),
                }
            })
            .collect();
        
        Ok(WorkflowMetrics {
            workflow_id,
            total_executions,
            completed_executions,
            completion_rate,
            average_duration_minutes: avg_duration,
            step_metrics,
        })
    }
}
```

### 3. Add Work Session Methods
```rust
impl TaskRepository for SqliteTaskRepository {
    async fn start_work_session(&self, agent_name: &str, task_code: &str) -> Result<i32> {
        // End any existing active sessions for this agent
        sqlx::query(
            r#"
            UPDATE work_sessions 
            SET is_active = 0, finished_at = CURRENT_TIMESTAMP
            WHERE agent_name = ? AND is_active = 1
            "#
        )
        .bind(agent_name)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Start new session
        let session_id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO work_sessions (agent_name, task_code)
            VALUES (?, ?)
            RETURNING id
            "#
        )
        .bind(agent_name)
        .bind(task_code)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(session_id)
    }
    
    async fn end_work_session(&self, session_id: i32, notes: Option<String>) -> Result<()> {
        let affected = sqlx::query(
            r#"
            UPDATE work_sessions 
            SET is_active = 0, 
                finished_at = CURRENT_TIMESTAMP,
                completion_notes = ?,
                total_active_minutes = (
                    (julianday(CURRENT_TIMESTAMP) - julianday(started_at)) * 24 * 60
                )
            WHERE id = ? AND is_active = 1
            "#
        )
        .bind(&notes)
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .rows_affected();
        
        if affected == 0 {
            return Err(TaskError::NotFound(format!("Active session {} not found", session_id)));
        }
        
        Ok(())
    }
    
    async fn update_work_session(&self, session_id: i32) -> Result<()> {
        // Update heartbeat - could track interruptions here
        let affected = sqlx::query(
            r#"
            UPDATE work_sessions 
            SET total_active_minutes = (
                (julianday(CURRENT_TIMESTAMP) - julianday(started_at)) * 24 * 60
            )
            WHERE id = ? AND is_active = 1
            "#
        )
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .rows_affected();
        
        if affected == 0 {
            return Err(TaskError::NotFound(format!("Active session {} not found", session_id)));
        }
        
        Ok(())
    }
    
    async fn get_active_sessions(&self) -> Result<Vec<WorkSession>> {
        let sessions = sqlx::query(
            r#"
            SELECT id, agent_name, task_code, started_at, finished_at,
                   total_active_minutes, interruptions, is_active, completion_notes
            FROM work_sessions
            WHERE is_active = 1
            ORDER BY started_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        sessions.into_iter()
            .map(|row| self.row_to_work_session(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_task_sessions(&self, task_code: &str) -> Result<Vec<WorkSession>> {
        let sessions = sqlx::query(
            r#"
            SELECT id, agent_name, task_code, started_at, finished_at,
                   total_active_minutes, interruptions, is_active, completion_notes
            FROM work_sessions
            WHERE task_code = ?
            ORDER BY started_at DESC
            "#
        )
        .bind(task_code)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        sessions.into_iter()
            .map(|row| self.row_to_work_session(row))
            .collect::<Result<Vec<_>>>()
    }
}
```

### 4. Add Helper Methods
```rust
impl SqliteTaskRepository {
    fn row_to_system_event(&self, row: SqliteRow) -> Result<SystemEvent> {
        let actor_type_str: String = row.get("actor_type");
        let actor_type = match actor_type_str.as_str() {
            "agent" => ActorType::Agent,
            "system" => ActorType::System,
            "human" => ActorType::Human,
            _ => ActorType::System,
        };
        
        let payload_json: String = row.get("payload");
        let payload = serde_json::from_str(&payload_json)
            .unwrap_or_else(|_| serde_json::json!({}));
        
        Ok(SystemEvent {
            id: row.get("id"),
            timestamp: row.get("timestamp"),
            event_type: row.get("event_type"),
            actor_type,
            actor_id: row.get("actor_id"),
            task_code: row.get("task_code"),
            payload,
            correlation_id: row.get("correlation_id"),
        })
    }
    
    fn row_to_work_session(&self, row: SqliteRow) -> Result<WorkSession> {
        let interruptions_json: Option<String> = row.get("interruptions");
        let interruptions = interruptions_json
            .and_then(|json| serde_json::from_str::<Vec<Interruption>>(&json).ok())
            .unwrap_or_default();
        
        Ok(WorkSession {
            id: row.get("id"),
            agent_name: row.get("agent_name"),
            task_code: row.get("task_code"),
            started_at: row.get("started_at"),
            finished_at: row.get("finished_at"),
            total_active_minutes: row.get("total_active_minutes"),
            interruptions,
            is_active: row.get("is_active"),
            completion_notes: row.get("completion_notes"),
        })
    }
}
```

## Files to Modify
- `database/src/sqlite.rs` - Add analytics method implementations
- `database/src/lib.rs` - Ensure all analytics types are exported

## Testing Requirements
1. Test event logging and querying
2. Test metric calculations with various time ranges
3. Test agent performance tracking
4. Test system-wide metrics
5. Test work session tracking
6. Test help request statistics
7. Test workflow metrics

## Performance Considerations
1. Event queries should use timestamp indexes
2. Aggregate queries may need optimization for large datasets
3. Consider materialized views for frequently accessed metrics
4. Work session updates should be lightweight
5. Consider caching system metrics

## Security Considerations
1. Validate all event payloads are valid JSON
2. Ensure time ranges are reasonable to prevent DOS
3. Rate limit metric queries if exposed via API
4. Sanitize all user-provided event data