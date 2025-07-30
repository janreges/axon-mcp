# PROTOCOL06: Implement Analytics and Metrics Handlers

## Objective
Implement all analytics and metrics protocol handlers, providing comprehensive system insights, performance metrics, and operational intelligence through the MCP protocol.

## Implementation Details

### 1. Extend Protocol Handler with Analytics Methods
In `mcp-protocol/src/handler.rs`, add analytics and metrics implementations:

```rust
// Add to the existing McpProtocolHandler implementation
impl<R: TaskRepository> McpProtocolHandler<R> {
    // ... existing methods ...
    
    // ===== Analytics & Metrics Methods =====
    
    async fn handle_get_task_metrics(&self, params: GetTaskMetricsParams) -> Result<TaskMetrics> {
        let since = params.since
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now() - Duration::days(7));
        
        let until = params.until
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now());
        
        // Get basic metrics
        let total_tasks = self.repository.count_tasks(Some(since), Some(until)).await?;
        let completed_tasks = self.repository.count_tasks_by_state(
            TaskState::Done,
            Some(since),
            Some(until),
        ).await?;
        
        // Get state distribution
        let state_distribution = self.get_state_distribution(since, until).await?;
        
        // Get completion time stats
        let completion_stats = self.repository
            .get_task_completion_stats(since, until)
            .await?;
        
        // Get priority distribution
        let priority_distribution = self.get_priority_distribution(since, until).await?;
        
        // Get failure analysis
        let failure_analysis = self.get_failure_analysis(since, until).await?;
        
        // Calculate velocity
        let velocity = self.calculate_velocity(since, until).await?;
        
        Ok(TaskMetrics {
            time_range: TimeRange { since, until },
            total_tasks,
            completed_tasks,
            completion_rate: if total_tasks > 0 {
                (completed_tasks as f64 / total_tasks as f64) * 100.0
            } else {
                0.0
            },
            state_distribution,
            average_completion_time_minutes: completion_stats.avg_minutes,
            median_completion_time_minutes: completion_stats.median_minutes,
            priority_distribution,
            failure_analysis,
            velocity,
            bottlenecks: self.identify_bottlenecks(since, until).await?,
        })
    }
    
    async fn handle_get_agent_metrics(&self, params: GetAgentMetricsParams) -> Result<AgentMetrics> {
        let agent = self.repository
            .get_agent(&params.agent_name)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", params.agent_name)))?;
        
        let since = params.since
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now() - Duration::days(30));
        
        // Get task statistics
        let task_stats = self.repository
            .get_agent_task_stats(&params.agent_name, since)
            .await?;
        
        // Get performance metrics
        let performance = self.calculate_agent_performance(&params.agent_name, since).await?;
        
        // Get collaboration stats
        let collaboration = self.get_collaboration_stats(&params.agent_name, since).await?;
        
        // Get capability utilization
        let capability_utilization = self.calculate_capability_utilization(&agent, since).await?;
        
        // Get help statistics
        let help_stats = self.get_agent_help_stats(&params.agent_name, since).await?;
        
        Ok(AgentMetrics {
            agent_name: params.agent_name,
            time_range: TimeRange { since, until: Utc::now() },
            tasks_completed: task_stats.completed,
            tasks_failed: task_stats.failed,
            tasks_in_progress: agent.current_load,
            average_task_duration_minutes: performance.avg_duration_minutes,
            success_rate: performance.success_rate,
            reputation_score: agent.reputation_score,
            reputation_trend: performance.reputation_trend,
            capability_utilization,
            collaboration_stats: collaboration,
            help_provided: help_stats.help_provided,
            help_requested: help_stats.help_requested,
            workload_history: self.get_workload_history(&params.agent_name, since).await?,
            specialization_effectiveness: self.calculate_specialization_effectiveness(&agent, since).await?,
        })
    }
    
    async fn handle_get_system_metrics(&self, params: GetSystemMetricsParams) -> Result<SystemMetrics> {
        let since = params.since
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now() - Duration::hours(24));
        
        // Get system health
        let health = self.calculate_system_health().await?;
        
        // Get throughput metrics
        let throughput = self.calculate_throughput(since).await?;
        
        // Get queue metrics
        let queue_metrics = self.get_queue_metrics().await?;
        
        // Get agent utilization
        let agent_utilization = self.calculate_overall_agent_utilization().await?;
        
        // Get workflow metrics
        let workflow_metrics = self.get_workflow_metrics(since).await?;
        
        // Get error rates
        let error_analysis = self.analyze_system_errors(since).await?;
        
        Ok(SystemMetrics {
            timestamp: Utc::now(),
            health_score: health.score,
            health_status: health.status,
            active_agents: health.active_agents,
            total_agents: health.total_agents,
            tasks_in_queue: queue_metrics.tasks_waiting,
            tasks_in_progress: queue_metrics.tasks_in_progress,
            tasks_completed_24h: throughput.completed_24h,
            throughput_per_hour: throughput.per_hour,
            average_queue_time_minutes: queue_metrics.avg_wait_minutes,
            agent_utilization_percentage: agent_utilization,
            workflow_metrics,
            error_rates: error_analysis,
            performance_indicators: self.get_performance_indicators().await?,
            recommendations: self.generate_system_recommendations(&health, &queue_metrics).await?,
        })
    }
    
    async fn handle_get_capability_coverage(&self, params: GetCapabilityCoverageParams) -> Result<CapabilityCoverage> {
        // Get all unique capabilities from tasks
        let task_capabilities = self.repository.get_all_task_capabilities().await?;
        
        // Get all agent capabilities
        let agent_capabilities = self.repository.get_all_agent_capabilities().await?;
        
        let mut coverage_map = std::collections::HashMap::new();
        
        for capability in &task_capabilities {
            let agents = self.repository
                .find_agents_by_capability(capability, 100)
                .await?;
            
            let coverage = CapabilityCoverageItem {
                capability: capability.clone(),
                agent_count: agents.len() as i32,
                agents: agents.into_iter().map(|a| a.agent_name).collect(),
                task_demand: self.count_tasks_requiring_capability(capability).await?,
                coverage_ratio: if agents.is_empty() {
                    0.0
                } else {
                    agents.len() as f64 / self.count_tasks_requiring_capability(capability).await? as f64
                },
                is_bottleneck: agents.len() < 2 && self.count_tasks_requiring_capability(capability).await? > 5,
            };
            
            coverage_map.insert(capability.clone(), coverage);
        }
        
        // Find gaps (capabilities needed but not covered)
        let gaps: Vec<_> = task_capabilities.into_iter()
            .filter(|cap| !agent_capabilities.contains(cap))
            .collect();
        
        // Find underutilized (capabilities available but not needed)
        let underutilized: Vec<_> = agent_capabilities.into_iter()
            .filter(|cap| !coverage_map.contains_key(cap))
            .collect();
        
        Ok(CapabilityCoverage {
            coverage_map,
            gaps,
            underutilized_capabilities: underutilized,
            recommendations: self.generate_capability_recommendations(&coverage_map).await?,
        })
    }
    
    async fn handle_get_performance_report(&self, params: GetPerformanceReportParams) -> Result<PerformanceReport> {
        let period = params.period.unwrap_or("week".to_string());
        let (since, until) = self.parse_period(&period)?;
        
        // Collect all metrics
        let task_metrics = self.handle_get_task_metrics(GetTaskMetricsParams {
            since: Some(since.to_rfc3339()),
            until: Some(until.to_rfc3339()),
        }).await?;
        
        let system_metrics = self.handle_get_system_metrics(GetSystemMetricsParams {
            since: Some(since.to_rfc3339()),
        }).await?;
        
        // Get top performers
        let top_agents = self.get_top_performing_agents(since, until, 5).await?;
        
        // Get problematic tasks
        let problematic_tasks = self.get_problematic_tasks(since, until).await?;
        
        // Get trends
        let trends = self.calculate_trends(since, until).await?;
        
        // Generate insights
        let insights = self.generate_insights(&task_metrics, &system_metrics, &trends).await?;
        
        Ok(PerformanceReport {
            report_id: format!("perf-{}", Utc::now().timestamp()),
            generated_at: Utc::now(),
            period: PeriodInfo { since, until, label: period },
            executive_summary: self.generate_executive_summary(&task_metrics, &system_metrics).await?,
            task_metrics,
            system_metrics,
            top_performing_agents: top_agents,
            problematic_tasks,
            trends,
            insights,
            recommendations: self.generate_performance_recommendations(&insights).await?,
        })
    }
    
    // Helper methods
    
    async fn get_state_distribution(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<StateCount>> {
        let states = vec![
            TaskState::Created,
            TaskState::InProgress,
            TaskState::Blocked,
            TaskState::Review,
            TaskState::Done,
            TaskState::Archived,
        ];
        
        let mut distribution = Vec::new();
        
        for state in states {
            let count = self.repository.count_tasks_by_state(state, Some(since), Some(until)).await?;
            distribution.push(StateCount {
                state: state.to_string(),
                count,
                percentage: 0.0, // Will be calculated after getting all counts
            });
        }
        
        let total: i32 = distribution.iter().map(|s| s.count).sum();
        for item in &mut distribution {
            item.percentage = if total > 0 {
                (item.count as f64 / total as f64) * 100.0
            } else {
                0.0
            };
        }
        
        Ok(distribution)
    }
    
    async fn get_priority_distribution(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<PriorityCount>> {
        self.repository.get_priority_distribution(since, until).await
    }
    
    async fn get_failure_analysis(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<FailureAnalysis> {
        let failed_tasks = self.repository.get_failed_tasks(since, until).await?;
        
        let mut failure_reasons = std::collections::HashMap::new();
        let mut failing_agents = std::collections::HashMap::new();
        
        for task in &failed_tasks {
            // Analyze failure events
            let events = self.repository.get_task_events(&task.code).await?;
            
            for event in events {
                if event.event_type == "task_failed" {
                    let reason = event.payload.get("reason")
                        .and_then(|r| r.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    *failure_reasons.entry(reason).or_insert(0) += 1;
                    *failing_agents.entry(event.actor_id.clone()).or_insert(0) += 1;
                }
            }
        }
        
        Ok(FailureAnalysis {
            total_failures: failed_tasks.len() as i32,
            failure_rate: self.calculate_failure_rate(since, until).await?,
            top_failure_reasons: failure_reasons.into_iter()
                .map(|(reason, count)| (reason, count))
                .collect(),
            most_failing_agents: failing_agents.into_iter()
                .map(|(agent, count)| (agent, count))
                .collect(),
            recovery_rate: self.calculate_recovery_rate(&failed_tasks).await?,
        })
    }
    
    async fn calculate_velocity(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<VelocityMetrics> {
        let days = (until - since).num_days() as f64;
        let weeks = days / 7.0;
        
        let completed = self.repository.count_tasks_by_state(
            TaskState::Done,
            Some(since),
            Some(until),
        ).await?;
        
        Ok(VelocityMetrics {
            tasks_per_day: completed as f64 / days,
            tasks_per_week: completed as f64 / weeks,
            trend: self.calculate_velocity_trend(since, until).await?,
        })
    }
    
    async fn identify_bottlenecks(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Vec<Bottleneck>> {
        let mut bottlenecks = Vec::new();
        
        // Long-running tasks
        let long_running = self.repository.get_long_running_tasks(Duration::days(3)).await?;
        if !long_running.is_empty() {
            bottlenecks.push(Bottleneck {
                bottleneck_type: "long_running_tasks".to_string(),
                description: format!("{} tasks running longer than 3 days", long_running.len()),
                impact: "high".to_string(),
                affected_items: long_running.into_iter().map(|t| t.code).collect(),
                recommendation: "Review and possibly decompose complex tasks".to_string(),
            });
        }
        
        // Blocked tasks
        let blocked_count = self.repository.count_tasks_by_state(
            TaskState::Blocked,
            None,
            None,
        ).await?;
        
        if blocked_count > 10 {
            bottlenecks.push(Bottleneck {
                bottleneck_type: "blocked_tasks".to_string(),
                description: format!("{} tasks currently blocked", blocked_count),
                impact: "high".to_string(),
                affected_items: vec![],
                recommendation: "Prioritize resolving blockers".to_string(),
            });
        }
        
        // Capability bottlenecks
        let capability_gaps = self.repository.get_capability_gaps().await?;
        for (capability, demand) in capability_gaps {
            if demand > 5 {
                bottlenecks.push(Bottleneck {
                    bottleneck_type: "capability_shortage".to_string(),
                    description: format!("High demand for {} capability ({} tasks waiting)", capability, demand),
                    impact: "medium".to_string(),
                    affected_items: vec![capability],
                    recommendation: "Consider training agents or hiring for this capability".to_string(),
                });
            }
        }
        
        Ok(bottlenecks)
    }
    
    async fn calculate_system_health(&self) -> Result<SystemHealth> {
        let agents = self.repository.list_agents(AgentFilter::default()).await?;
        let active_agents = agents.iter().filter(|a| a.is_available()).count();
        
        let queue_size = self.repository.count_tasks_by_state(
            TaskState::Created,
            None,
            None,
        ).await?;
        
        let blocked_tasks = self.repository.count_tasks_by_state(
            TaskState::Blocked,
            None,
            None,
        ).await?;
        
        // Calculate health score (0-100)
        let mut score = 100.0;
        
        // Penalize for inactive agents
        let agent_availability = active_agents as f64 / agents.len() as f64;
        if agent_availability < 0.5 {
            score -= 20.0;
        } else if agent_availability < 0.8 {
            score -= 10.0;
        }
        
        // Penalize for large queue
        if queue_size > active_agents as i32 * 10 {
            score -= 15.0;
        } else if queue_size > active_agents as i32 * 5 {
            score -= 5.0;
        }
        
        // Penalize for blocked tasks
        if blocked_tasks > 20 {
            score -= 20.0;
        } else if blocked_tasks > 10 {
            score -= 10.0;
        }
        
        let status = match score {
            s if s >= 90.0 => "healthy".to_string(),
            s if s >= 70.0 => "degraded".to_string(),
            s if s >= 50.0 => "warning".to_string(),
            _ => "critical".to_string(),
        };
        
        Ok(SystemHealth {
            score,
            status,
            active_agents: active_agents as i32,
            total_agents: agents.len() as i32,
            checks_passed: vec![
                ("agent_availability".to_string(), agent_availability >= 0.8),
                ("queue_manageable".to_string(), queue_size <= active_agents as i32 * 5),
                ("blockers_under_control".to_string(), blocked_tasks <= 10),
            ],
        })
    }
    
    async fn generate_insights(
        &self,
        task_metrics: &TaskMetrics,
        system_metrics: &SystemMetrics,
        trends: &TrendAnalysis,
    ) -> Result<Vec<Insight>> {
        let mut insights = Vec::new();
        
        // Task completion insights
        if task_metrics.completion_rate < 70.0 {
            insights.push(Insight {
                insight_type: "performance".to_string(),
                severity: "warning".to_string(),
                message: format!("Task completion rate is {:.1}%, below target of 70%", task_metrics.completion_rate),
                recommendation: "Review task assignment and agent workload".to_string(),
                supporting_data: serde_json::json!({
                    "completion_rate": task_metrics.completion_rate,
                    "completed": task_metrics.completed_tasks,
                    "total": task_metrics.total_tasks,
                }),
            });
        }
        
        // Velocity insights
        if trends.velocity_trend < -10.0 {
            insights.push(Insight {
                insight_type: "trend".to_string(),
                severity: "warning".to_string(),
                message: "Task completion velocity declining".to_string(),
                recommendation: "Investigate causes of slowdown".to_string(),
                supporting_data: serde_json::json!({
                    "velocity_trend": trends.velocity_trend,
                }),
            });
        }
        
        // System health insights
        if system_metrics.health_score < 80.0 {
            insights.push(Insight {
                insight_type: "health".to_string(),
                severity: "high".to_string(),
                message: format!("System health score is {:.1}, indicating issues", system_metrics.health_score),
                recommendation: "Address system bottlenecks and agent availability".to_string(),
                supporting_data: serde_json::json!({
                    "health_score": system_metrics.health_score,
                    "active_agents": system_metrics.active_agents,
                }),
            });
        }
        
        Ok(insights)
    }
    
    async fn generate_executive_summary(
        &self,
        task_metrics: &TaskMetrics,
        system_metrics: &SystemMetrics,
    ) -> Result<String> {
        Ok(format!(
            "System processed {} tasks with {:.1}% completion rate. \
             {} agents active with {:.1}% utilization. \
             Average task completion time: {} minutes. \
             System health: {} ({:.0}/100)",
            task_metrics.total_tasks,
            task_metrics.completion_rate,
            system_metrics.active_agents,
            system_metrics.agent_utilization_percentage,
            task_metrics.average_completion_time_minutes,
            system_metrics.health_status,
            system_metrics.health_score,
        ))
    }
}
```

### 2. Add Analytics Parameters and Response Types
In `mcp-protocol/src/params.rs`:

```rust
// Analytics Parameters
#[derive(Debug, Clone, Deserialize)]
pub struct GetTaskMetricsParams {
    pub since: Option<String>, // ISO 8601
    pub until: Option<String>, // ISO 8601
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetAgentMetricsParams {
    pub agent_name: String,
    pub since: Option<String>, // ISO 8601
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetSystemMetricsParams {
    pub since: Option<String>, // ISO 8601
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetCapabilityCoverageParams {
    pub include_underutilized: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetPerformanceReportParams {
    pub period: Option<String>, // day, week, month, custom
    pub since: Option<String>, // For custom period
    pub until: Option<String>, // For custom period
}

// Response Types
#[derive(Debug, Clone, Serialize)]
pub struct TaskMetrics {
    pub time_range: TimeRange,
    pub total_tasks: i32,
    pub completed_tasks: i32,
    pub completion_rate: f64,
    pub state_distribution: Vec<StateCount>,
    pub average_completion_time_minutes: f64,
    pub median_completion_time_minutes: f64,
    pub priority_distribution: Vec<PriorityCount>,
    pub failure_analysis: FailureAnalysis,
    pub velocity: VelocityMetrics,
    pub bottlenecks: Vec<Bottleneck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentMetrics {
    pub agent_name: String,
    pub time_range: TimeRange,
    pub tasks_completed: i32,
    pub tasks_failed: i32,
    pub tasks_in_progress: i32,
    pub average_task_duration_minutes: f64,
    pub success_rate: f64,
    pub reputation_score: f64,
    pub reputation_trend: f64,
    pub capability_utilization: Vec<CapabilityUtilization>,
    pub collaboration_stats: CollaborationStats,
    pub help_provided: i32,
    pub help_requested: i32,
    pub workload_history: Vec<WorkloadPoint>,
    pub specialization_effectiveness: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub health_score: f64,
    pub health_status: String,
    pub active_agents: i32,
    pub total_agents: i32,
    pub tasks_in_queue: i32,
    pub tasks_in_progress: i32,
    pub tasks_completed_24h: i32,
    pub throughput_per_hour: f64,
    pub average_queue_time_minutes: f64,
    pub agent_utilization_percentage: f64,
    pub workflow_metrics: WorkflowMetrics,
    pub error_rates: ErrorAnalysis,
    pub performance_indicators: Vec<KPI>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeRange {
    pub since: DateTime<Utc>,
    pub until: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateCount {
    pub state: String,
    pub count: i32,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriorityCount {
    pub priority: i32,
    pub count: i32,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FailureAnalysis {
    pub total_failures: i32,
    pub failure_rate: f64,
    pub top_failure_reasons: Vec<(String, i32)>,
    pub most_failing_agents: Vec<(String, i32)>,
    pub recovery_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VelocityMetrics {
    pub tasks_per_day: f64,
    pub tasks_per_week: f64,
    pub trend: f64, // Percentage change
}

#[derive(Debug, Clone, Serialize)]
pub struct Bottleneck {
    pub bottleneck_type: String,
    pub description: String,
    pub impact: String,
    pub affected_items: Vec<String>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityUtilization {
    pub capability: String,
    pub tasks_handled: i32,
    pub utilization_percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollaborationStats {
    pub handoffs_sent: i32,
    pub handoffs_received: i32,
    pub messages_sent: i32,
    pub help_collaborations: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkloadPoint {
    pub timestamp: DateTime<Utc>,
    pub load: i32,
    pub capacity: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemHealth {
    pub score: f64,
    pub status: String,
    pub active_agents: i32,
    pub total_agents: i32,
    pub checks_passed: Vec<(String, bool)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityCoverage {
    pub coverage_map: std::collections::HashMap<String, CapabilityCoverageItem>,
    pub gaps: Vec<String>,
    pub underutilized_capabilities: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityCoverageItem {
    pub capability: String,
    pub agent_count: i32,
    pub agents: Vec<String>,
    pub task_demand: i32,
    pub coverage_ratio: f64,
    pub is_bottleneck: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerformanceReport {
    pub report_id: String,
    pub generated_at: DateTime<Utc>,
    pub period: PeriodInfo,
    pub executive_summary: String,
    pub task_metrics: TaskMetrics,
    pub system_metrics: SystemMetrics,
    pub top_performing_agents: Vec<AgentPerformance>,
    pub problematic_tasks: Vec<ProblematicTask>,
    pub trends: TrendAnalysis,
    pub insights: Vec<Insight>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrendAnalysis {
    pub velocity_trend: f64,
    pub quality_trend: f64,
    pub efficiency_trend: f64,
    pub workload_trend: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Insight {
    pub insight_type: String,
    pub severity: String,
    pub message: String,
    pub recommendation: String,
    pub supporting_data: serde_json::Value,
}
```

### 3. Create Analytics Dashboard Service
In `mcp-protocol/src/services/analytics_dashboard.rs`:

```rust
use core::{models::*, repository::TaskRepository};
use std::sync::Arc;

pub struct AnalyticsDashboardService<R: TaskRepository> {
    repository: Arc<R>,
}

impl<R: TaskRepository> AnalyticsDashboardService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
    
    /// Get real-time dashboard data
    pub async fn get_dashboard_data(&self) -> Result<DashboardData> {
        let now = Utc::now();
        let day_ago = now - Duration::days(1);
        let week_ago = now - Duration::days(7);
        
        // Get current state
        let current_state = CurrentSystemState {
            timestamp: now,
            active_tasks: self.repository.count_tasks_by_state(TaskState::InProgress, None, None).await?,
            queued_tasks: self.repository.count_tasks_by_state(TaskState::Created, None, None).await?,
            blocked_tasks: self.repository.count_tasks_by_state(TaskState::Blocked, None, None).await?,
            active_agents: self.count_active_agents().await?,
            total_agents: self.repository.list_agents(AgentFilter::default()).await?.len() as i32,
        };
        
        // Get 24h metrics
        let daily_metrics = DailyMetrics {
            tasks_created: self.repository.count_tasks(Some(day_ago), Some(now)).await?,
            tasks_completed: self.repository.count_tasks_by_state(
                TaskState::Done,
                Some(day_ago),
                Some(now),
            ).await?,
            average_completion_time: self.calculate_avg_completion_time(day_ago, now).await?,
            help_requests: self.count_help_requests(day_ago, now).await?,
            handoffs: self.count_handoffs(day_ago, now).await?,
        };
        
        // Get trending data
        let trends = self.calculate_dashboard_trends(week_ago, now).await?;
        
        // Get alerts
        let alerts = self.generate_dashboard_alerts(&current_state).await?;
        
        Ok(DashboardData {
            current_state,
            daily_metrics,
            trends,
            alerts,
            last_updated: now,
        })
    }
    
    /// Generate time series data for charts
    pub async fn get_time_series_data(
        &self,
        metric: &str,
        granularity: &str,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let interval = match granularity {
            "hour" => Duration::hours(1),
            "day" => Duration::days(1),
            "week" => Duration::weeks(1),
            _ => Duration::days(1),
        };
        
        let mut points = Vec::new();
        let mut current = since;
        
        while current < until {
            let next = current + interval;
            let value = match metric {
                "task_completions" => {
                    self.repository.count_tasks_by_state(
                        TaskState::Done,
                        Some(current),
                        Some(next),
                    ).await? as f64
                }
                "active_tasks" => {
                    self.repository.count_tasks_by_state(
                        TaskState::InProgress,
                        Some(current),
                        Some(next),
                    ).await? as f64
                }
                "agent_utilization" => {
                    self.calculate_point_in_time_utilization(current).await?
                }
                "queue_size" => {
                    self.repository.count_tasks_by_state(
                        TaskState::Created,
                        Some(current),
                        Some(next),
                    ).await? as f64
                }
                _ => 0.0,
            };
            
            points.push(TimeSeriesPoint {
                timestamp: current,
                value,
                label: current.format("%Y-%m-%d %H:%M").to_string(),
            });
            
            current = next;
        }
        
        Ok(points)
    }
    
    /// Get leaderboard data
    pub async fn get_agent_leaderboard(
        &self,
        period: &str,
        limit: i32,
    ) -> Result<Vec<LeaderboardEntry>> {
        let (since, _) = self.parse_period(period)?;
        
        let agents = self.repository.list_agents(AgentFilter::default()).await?;
        let mut entries = Vec::new();
        
        for agent in agents {
            let stats = self.repository.get_agent_task_stats(&agent.name, since).await?;
            
            let score = stats.completed as f64 * 10.0 +
                       stats.helped_others as f64 * 5.0 -
                       stats.failed as f64 * 2.0;
            
            entries.push(LeaderboardEntry {
                agent_name: agent.name,
                score,
                tasks_completed: stats.completed,
                success_rate: if stats.completed + stats.failed > 0 {
                    stats.completed as f64 / (stats.completed + stats.failed) as f64
                } else {
                    0.0
                },
                help_provided: stats.helped_others,
                reputation: agent.reputation_score,
                rank: 0, // Will be set after sorting
            });
        }
        
        // Sort by score and assign ranks
        entries.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        for (idx, entry) in entries.iter_mut().enumerate() {
            entry.rank = (idx + 1) as i32;
        }
        
        entries.truncate(limit as usize);
        Ok(entries)
    }
    
    async fn count_active_agents(&self) -> Result<i32> {
        let agents = self.repository.list_agents(AgentFilter::default()).await?;
        Ok(agents.iter().filter(|a| a.is_available()).count() as i32)
    }
    
    async fn calculate_dashboard_trends(
        &self,
        since: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<DashboardTrends> {
        // This would calculate week-over-week trends
        // Simplified for example
        Ok(DashboardTrends {
            task_completion_trend: 5.2,
            agent_utilization_trend: -2.1,
            queue_size_trend: 12.5,
            error_rate_trend: -8.3,
        })
    }
    
    async fn generate_dashboard_alerts(
        &self,
        state: &CurrentSystemState,
    ) -> Result<Vec<DashboardAlert>> {
        let mut alerts = Vec::new();
        
        // Check for high queue
        if state.queued_tasks > state.active_agents * 20 {
            alerts.push(DashboardAlert {
                alert_type: "high_queue".to_string(),
                severity: "warning".to_string(),
                message: format!("Task queue is very high: {} tasks waiting", state.queued_tasks),
                action: "Consider scaling up agents".to_string(),
            });
        }
        
        // Check for many blocked tasks
        if state.blocked_tasks > 10 {
            alerts.push(DashboardAlert {
                alert_type: "blocked_tasks".to_string(),
                severity: "warning".to_string(),
                message: format!("{} tasks are blocked", state.blocked_tasks),
                action: "Review and resolve blockers".to_string(),
            });
        }
        
        // Check agent availability
        let agent_availability = state.active_agents as f64 / state.total_agents as f64;
        if agent_availability < 0.5 {
            alerts.push(DashboardAlert {
                alert_type: "low_availability".to_string(),
                severity: "high".to_string(),
                message: "Less than 50% of agents are active".to_string(),
                action: "Check agent health and availability".to_string(),
            });
        }
        
        Ok(alerts)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardData {
    pub current_state: CurrentSystemState,
    pub daily_metrics: DailyMetrics,
    pub trends: DashboardTrends,
    pub alerts: Vec<DashboardAlert>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentSystemState {
    pub timestamp: DateTime<Utc>,
    pub active_tasks: i32,
    pub queued_tasks: i32,
    pub blocked_tasks: i32,
    pub active_agents: i32,
    pub total_agents: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyMetrics {
    pub tasks_created: i32,
    pub tasks_completed: i32,
    pub average_completion_time: f64,
    pub help_requests: i32,
    pub handoffs: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardTrends {
    pub task_completion_trend: f64,
    pub agent_utilization_trend: f64,
    pub queue_size_trend: f64,
    pub error_rate_trend: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardAlert {
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LeaderboardEntry {
    pub agent_name: String,
    pub score: f64,
    pub tasks_completed: i32,
    pub success_rate: f64,
    pub help_provided: i32,
    pub reputation: f64,
    pub rank: i32,
}
```

## Files to Create/Modify
- `mcp-protocol/src/handler.rs` - Add analytics handler methods
- `mcp-protocol/src/params.rs` - Add analytics parameter types
- `mcp-protocol/src/services/analytics_dashboard.rs` - Dashboard service
- `mcp-protocol/src/router.rs` - Add analytics method routing

## Testing Requirements
1. Test metric calculations
2. Test time series data generation
3. Test performance report generation
4. Test bottleneck identification
5. Test trend analysis
6. Test dashboard real-time updates
7. Test leaderboard calculations

## Notes
- Comprehensive metrics for tasks, agents, and system
- Real-time dashboard support
- Performance insights and recommendations
- Trend analysis for proactive management
- Capability coverage analysis
- Bottleneck identification