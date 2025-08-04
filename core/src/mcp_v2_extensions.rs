use crate::{Result, Task, TaskError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// MCP v2 Extensions for Multi-Agent Coordination
///
/// This module provides the core business logic for MCP v2 features optimized
/// for local SQLite-based deployments with 8-20 AI agents.
/// Work discovery response for agents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiscoverWorkResponse {
    /// Tasks available for the agent
    TasksAvailable(Vec<Task>),
    /// No tasks currently available
    NoTasksAvailable,
    /// Agent must complete a prerequisite action first
    PrerequisiteActionRequired(PrerequisiteAction),
}

/// Prerequisite action that must be completed before getting work
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrerequisiteAction {
    /// Type of action required
    pub action_type: String,
    /// Human-readable description
    pub message: String,
    /// MCP function to call to complete the action
    pub respond_via_function: String,
    /// Additional context data
    pub context: serde_json::Value,
}

/// Work discovery configuration for local AI agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkDiscoveryConfig {
    /// Maximum number of tasks to return
    pub max_tasks: i32,
    /// Minimum priority score to consider
    pub min_priority: f64,
    /// Maximum failure count to consider
    pub max_failures: i32,
    /// Age bonus factor (increases priority for older tasks)
    pub age_bonus_factor: f64,
}

impl Default for WorkDiscoveryConfig {
    fn default() -> Self {
        Self {
            max_tasks: 5,
            min_priority: 0.0,
            max_failures: 2,
            age_bonus_factor: 0.1, // 10% bonus per hour
        }
    }
}

/// Task claim result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClaimResult {
    /// Task successfully claimed
    Success(Task),
    /// Task already claimed by another agent
    AlreadyClaimed { task_id: i32, claimed_by: String },
    /// Agent lacks required capabilities
    InsufficientCapabilities {
        task_id: i32,
        required: Vec<String>,
        agent_has: Vec<String>,
    },
    /// Task is quarantined due to too many failures
    Quarantined {
        task_id: i32,
        reason: String,
        retry_after: DateTime<Utc>,
    },
}

/// Agent capability matcher for local work assignment
#[derive(Debug, Clone)]
pub struct CapabilityMatcher {
    /// Exact match requirements (agent must have all of these)
    exact_match_weight: f64,
    /// Partial match weight (agent has some capabilities)
    partial_match_weight: f64,
    /// Specialization bonus (agent has deep expertise)
    specialization_bonus: f64,
}

impl Default for CapabilityMatcher {
    fn default() -> Self {
        Self {
            exact_match_weight: 1.0,
            partial_match_weight: 0.5,
            specialization_bonus: 0.2,
        }
    }
}

impl CapabilityMatcher {
    /// Calculate capability match score between agent and task
    pub fn calculate_match_score(
        &self,
        task_requirements: &[String],
        agent_capabilities: &[String],
        agent_specializations: &[String],
    ) -> f64 {
        if task_requirements.is_empty() {
            return 1.0; // No requirements = perfect match
        }

        let mut score = 0.0;
        let mut matched_count = 0;

        // Check exact matches
        for requirement in task_requirements {
            if agent_capabilities.contains(requirement) {
                matched_count += 1;
                score += self.exact_match_weight;

                // Bonus for specialization
                if agent_specializations.contains(requirement) {
                    score += self.specialization_bonus;
                }
            }
        }

        // Apply partial match penalty if not all requirements met
        if matched_count < task_requirements.len() {
            let match_ratio = matched_count as f64 / task_requirements.len() as f64;
            score *= match_ratio * self.partial_match_weight;
        }

        score / task_requirements.len() as f64
    }

    /// Check if agent meets minimum capability requirements
    pub fn meets_requirements(
        &self,
        task_requirements: &[String],
        agent_capabilities: &[String],
    ) -> bool {
        if task_requirements.is_empty() {
            return true;
        }

        // For local AI agents, require at least 50% capability match
        let match_count = task_requirements
            .iter()
            .filter(|req| agent_capabilities.contains(req))
            .count();

        let match_ratio = match_count as f64 / task_requirements.len() as f64;
        match_ratio >= 0.5
    }
}

/// Task priority calculator with staleness factor
#[derive(Debug, Clone)]
pub struct PriorityCalculator {
    config: WorkDiscoveryConfig,
}

impl PriorityCalculator {
    pub fn new(config: WorkDiscoveryConfig) -> Self {
        Self { config }
    }

    /// Calculate effective priority with age and failure adjustments
    pub fn calculate_effective_priority(&self, task: &Task) -> f64 {
        let mut priority = task.priority_score;

        // Apply age bonus (tasks get more important over time)
        let age_hours = (Utc::now() - task.inserted_at).num_hours() as f64;
        let age_bonus = age_hours * self.config.age_bonus_factor;
        priority += age_bonus;

        // Apply failure penalty (failed tasks become less attractive)
        let failure_penalty = task.failure_count as f64 * 0.5;
        priority -= failure_penalty;

        // Ensure priority stays within reasonable bounds
        priority.clamp(0.0, 20.0)
    }

    /// Check if task should be considered for assignment
    pub fn should_consider_task(&self, task: &Task) -> bool {
        // Skip quarantined tasks (would be handled by circuit breaker)
        if task.failure_count > self.config.max_failures {
            return false;
        }

        // Skip tasks below minimum priority threshold
        let effective_priority = self.calculate_effective_priority(task);
        effective_priority >= self.config.min_priority
    }
}

/// Simple work session tracker for local AI agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleWorkSession {
    pub task_id: i32,
    pub agent_name: String,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_active: bool,
}

impl SimpleWorkSession {
    pub fn new(task_id: i32, agent_name: String) -> Self {
        let now = Utc::now();
        Self {
            task_id,
            agent_name,
            started_at: now,
            last_activity: now,
            is_active: true,
        }
    }

    /// Update activity timestamp (heartbeat)
    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Check if session has timed out
    pub fn has_timed_out(&self, timeout_minutes: i64) -> bool {
        if !self.is_active {
            return false;
        }

        let elapsed = Utc::now() - self.last_activity;
        elapsed.num_minutes() > timeout_minutes
    }

    /// End the work session
    pub fn end_session(&mut self) {
        self.is_active = false;
        self.last_activity = Utc::now();
    }
}

/// Agent workload tracker for load balancing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorkload {
    pub agent_name: String,
    pub active_tasks: Vec<i32>,
    pub max_concurrent_tasks: i32,
    pub current_load_score: f64,
    pub last_heartbeat: DateTime<Utc>,
}

impl AgentWorkload {
    pub fn new(agent_name: String, max_concurrent_tasks: i32) -> Self {
        Self {
            agent_name,
            active_tasks: Vec::new(),
            max_concurrent_tasks,
            current_load_score: 0.0,
            last_heartbeat: Utc::now(),
        }
    }

    /// Check if agent can accept more work
    pub fn can_accept_work(&self) -> bool {
        self.active_tasks.len() < self.max_concurrent_tasks as usize
            && self.current_load_score < 0.9 // 90% capacity limit
    }

    /// Add task to agent's workload
    pub fn add_task(&mut self, task_id: i32) -> Result<()> {
        if !self.can_accept_work() {
            return Err(TaskError::Internal(format!(
                "Agent {} is at capacity",
                self.agent_name
            )));
        }

        self.active_tasks.push(task_id);
        self.update_load_score();
        Ok(())
    }

    /// Remove task from agent's workload
    pub fn remove_task(&mut self, task_id: i32) {
        self.active_tasks.retain(|&id| id != task_id);
        self.update_load_score();
    }

    /// Update load score based on current workload
    fn update_load_score(&mut self) {
        self.current_load_score = self.active_tasks.len() as f64 / self.max_concurrent_tasks as f64;
    }

    /// Update heartbeat timestamp
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
    }

    /// Check if agent is responsive (heartbeat within threshold)
    pub fn is_responsive(&self, timeout_minutes: i64) -> bool {
        let elapsed = Utc::now() - self.last_heartbeat;
        elapsed.num_minutes() <= timeout_minutes
    }
}

/// Local knowledge management for agent coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleKnowledgeEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub tags: Vec<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub confidence: f64,
}

impl SimpleKnowledgeEntry {
    pub fn new(
        key: String,
        value: serde_json::Value,
        created_by: String,
        tags: Vec<String>,
        confidence: Option<f64>,
    ) -> Self {
        Self {
            key,
            value,
            tags,
            created_by,
            created_at: Utc::now(),
            confidence: confidence.unwrap_or(0.8),
        }
    }

    /// Check if this knowledge is relevant to given tags
    pub fn is_relevant_to(&self, query_tags: &[String]) -> bool {
        if query_tags.is_empty() {
            return true;
        }

        // Check if any query tags match knowledge tags
        query_tags.iter().any(|tag| self.tags.contains(tag))
    }

    /// Check if knowledge is recent enough to be useful
    pub fn is_recent(&self, max_age_hours: i64) -> bool {
        let age = Utc::now() - self.created_at;
        age.num_hours() <= max_age_hours
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_matcher() {
        let matcher = CapabilityMatcher::default();

        let task_reqs = vec!["rust".to_string(), "database".to_string()];
        let agent_caps = vec![
            "rust".to_string(),
            "database".to_string(),
            "testing".to_string(),
        ];
        let agent_specs = vec!["rust".to_string()];

        let score = matcher.calculate_match_score(&task_reqs, &agent_caps, &agent_specs);
        assert!(score > 1.0); // Should get bonus for specialization

        assert!(matcher.meets_requirements(&task_reqs, &agent_caps));
    }

    #[test]
    fn test_priority_calculator() {
        let config = WorkDiscoveryConfig::default();
        let calc = PriorityCalculator::new(config);

        let mut task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test".to_string(),
            description: "Test task".to_string(),
            owner_agent_name: Some("test-agent".to_string()),
            state: crate::models::TaskState::Created,
            inserted_at: Utc::now() - chrono::Duration::hours(2),
            done_at: None,
            claimed_at: None,
            workflow_definition_id: None,
            workflow_cursor: None,
            priority_score: 5.0,
            parent_task_id: None,
            failure_count: 0,
            required_capabilities: vec![],
            estimated_effort: None,
            confidence_threshold: 0.8,
        };

        let priority = calc.calculate_effective_priority(&task);
        assert!(priority > 5.0); // Should get age bonus

        task.failure_count = 3;
        assert!(!calc.should_consider_task(&task)); // Too many failures
    }

    #[test]
    fn test_work_session() {
        let mut session = SimpleWorkSession::new(1, "test-agent".to_string());
        assert!(session.is_active);

        session.update_activity();
        assert!(!session.has_timed_out(60)); // Within 1 hour

        session.end_session();
        assert!(!session.is_active);
    }

    #[test]
    fn test_agent_workload() {
        let mut workload = AgentWorkload::new("test-agent".to_string(), 3);
        assert!(workload.can_accept_work());

        workload.add_task(1).unwrap();
        workload.add_task(2).unwrap();
        workload.add_task(3).unwrap();

        assert!(!workload.can_accept_work()); // At capacity

        workload.remove_task(1);
        assert!(workload.can_accept_work());
    }

    #[test]
    fn test_knowledge_entry() {
        let entry = SimpleKnowledgeEntry::new(
            "test-key".to_string(),
            serde_json::json!({"info": "test"}),
            "test-agent".to_string(),
            vec!["testing".to_string()],
            Some(0.9),
        );

        assert!(entry.is_relevant_to(&["testing".to_string()]));
        assert!(!entry.is_relevant_to(&["other".to_string()]));
        assert!(entry.is_recent(24)); // Within 24 hours
    }
}
