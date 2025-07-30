//! Mock implementation of TaskRepository trait
//! 
//! Provides a thread-safe mock repository with:
//! - Error injection capabilities
//! - Call tracking for verification
//! - Realistic behavior simulation

use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicI32, Ordering}};
use parking_lot::Mutex;
use async_trait::async_trait;
use chrono::Utc;
use task_core::{
    Task, TaskState, TaskRepository, TaskError, Result, NewTask, UpdateTask, TaskFilter,
    repository::RepositoryStats
};

/// Mock implementation of TaskRepository for testing
/// 
/// Features:
/// - Thread-safe concurrent access
/// - Error injection for failure testing
/// - Call history tracking for verification
/// - Realistic behavior simulation
pub struct MockTaskRepository {
    tasks: Arc<Mutex<HashMap<i32, Task>>>,
    next_id: Arc<AtomicI32>,
    error_injection: Arc<Mutex<Option<TaskError>>>,
    call_history: Arc<Mutex<Vec<String>>>,
}

impl Default for MockTaskRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTaskRepository {
    /// Create a new empty mock repository
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicI32::new(1)),
            error_injection: Arc::new(Mutex::new(None)),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create mock repository with pre-populated tasks
    pub fn with_tasks(tasks: Vec<Task>) -> Self {
        let mut task_map = HashMap::new();
        let mut max_id = 0;
        
        for task in tasks {
            if task.id > max_id {
                max_id = task.id;
            }
            task_map.insert(task.id, task);
        }
        
        Self {
            tasks: Arc::new(Mutex::new(task_map)),
            next_id: Arc::new(AtomicI32::new(max_id + 1)),
            error_injection: Arc::new(Mutex::new(None)),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create mock repository with specific starting ID
    pub fn with_next_id(next_id: i32) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicI32::new(next_id)),
            error_injection: Arc::new(Mutex::new(None)),
            call_history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Inject error for next operation
    pub fn inject_error(&self, error: TaskError) {
        *self.error_injection.lock() = Some(error);
    }

    /// Clear error injection
    pub fn clear_error(&self) {
        *self.error_injection.lock() = None;
    }

    /// Get history of called methods
    pub fn call_history(&self) -> Vec<String> {
        self.call_history.lock().clone()
    }

    /// Clear call history
    pub fn clear_history(&self) {
        self.call_history.lock().clear();
    }

    /// Assert method was called
    pub fn assert_called(&self, method: &str) {
        let history = self.call_history.lock();
        assert!(
            history.iter().any(|call| call.contains(method)),
            "Method '{}' was not called. Call history: {:?}",
            method,
            *history
        );
    }

    /// Check if an error should be injected, consuming it if so
    fn check_error_injection(&self) -> Result<()> {
        let mut error_opt = self.error_injection.lock();
        if let Some(error) = error_opt.take() {
            return Err(error);
        }
        Ok(())
    }

    /// Record method call in history
    fn record_call(&self, method: &str) {
        self.call_history.lock().push(format!("{method}()"));
    }

    /// Record method call with parameters in history
    fn record_call_with_params(&self, method: &str, params: &str) {
        self.call_history.lock().push(format!("{method}({params})"));
    }
}

#[async_trait]
impl TaskRepository for MockTaskRepository {
    async fn create(&self, task: NewTask) -> Result<Task> {
        self.record_call_with_params("create", &format!("code={}", task.code));
        
        // Check for error injection
        self.check_error_injection()?;
        
        // Check for duplicate code
        let tasks = self.tasks.lock();
        if tasks.values().any(|t| t.code == task.code) {
            return Err(TaskError::DuplicateCode(task.code));
        }
        drop(tasks);
        
        // Create task with next ID
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let now = Utc::now();
        
        let new_task = Task::new(
            id,
            task.code,
            task.name,
            task.description,
            task.owner_agent_name,
            TaskState::Created,
            now,
            None,
        );
        
        // Store in HashMap
        self.tasks.lock().insert(id, new_task.clone());
        
        Ok(new_task)
    }
    
    async fn update(&self, id: i32, updates: UpdateTask) -> Result<Task> {
        self.record_call_with_params("update", &format!("id={id}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&id).ok_or_else(|| TaskError::NotFound(id.to_string()))?;
        
        // Apply updates
        if let Some(name) = updates.name {
            task.name = name;
        }
        if let Some(description) = updates.description {
            task.description = description;
        }
        if let Some(owner) = updates.owner_agent_name {
            task.owner_agent_name = Some(owner);
        }
        
        Ok(task.clone())
    }
    
    async fn set_state(&self, id: i32, state: TaskState) -> Result<Task> {
        self.record_call_with_params("set_state", &format!("id={id}, state={state}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&id).ok_or_else(|| TaskError::NotFound(id.to_string()))?;
        
        // Validate state transition
        if !task.can_transition_to(state) {
            return Err(TaskError::InvalidStateTransition(task.state, state));
        }
        
        task.state = state;
        
        // Set completion timestamp if moving to Done
        if state == TaskState::Done {
            task.done_at = Some(Utc::now());
        }
        
        Ok(task.clone())
    }
    
    async fn get_by_id(&self, id: i32) -> Result<Option<Task>> {
        self.record_call_with_params("get_by_id", &format!("id={id}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        Ok(tasks.get(&id).cloned())
    }
    
    async fn get_by_code(&self, code: &str) -> Result<Option<Task>> {
        self.record_call_with_params("get_by_code", &format!("code={code}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        Ok(tasks.values().find(|t| t.code == code).cloned())
    }
    
    async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>> {
        self.record_call("list");
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        let mut result: Vec<Task> = tasks.values()
            .filter(|task| {
                // Filter by owner
                if let Some(ref owner) = filter.owner {
                    if task.owner_agent_name.as_deref() != Some(owner) {
                        return false;
                    }
                }
                
                // Filter by state
                if let Some(state) = filter.state {
                    if task.state != state {
                        return false;
                    }
                }
                
                // Filter by date range
                if let Some(date_from) = filter.date_from {
                    if task.inserted_at < date_from {
                        return false;
                    }
                }
                
                if let Some(date_to) = filter.date_to {
                    if task.inserted_at > date_to {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // Sort by creation date (most recent first)
        result.sort_by(|a, b| b.inserted_at.cmp(&a.inserted_at));
        
        // Apply pagination
        if let Some(offset) = filter.offset {
            if offset as usize >= result.len() {
                return Ok(Vec::new());
            }
            result = result.into_iter().skip(offset as usize).collect();
        }
        
        if let Some(limit) = filter.limit {
            result.truncate(limit as usize);
        }
        
        Ok(result)
    }
    
    async fn assign(&self, id: i32, new_owner: &str) -> Result<Task> {
        self.record_call_with_params("assign", &format!("id={id}, owner={new_owner}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        // Validate owner name is not empty
        if new_owner.trim().is_empty() {
            return Err(TaskError::Validation("Owner name cannot be empty".to_string()));
        }
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&id).ok_or_else(|| TaskError::NotFound(id.to_string()))?;
        
        task.owner_agent_name = Some(new_owner.to_string());
        
        Ok(task.clone())
    }
    
    async fn archive(&self, id: i32) -> Result<Task> {
        self.record_call_with_params("archive", &format!("id={id}"));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&id).ok_or_else(|| TaskError::NotFound(id.to_string()))?;
        
        // Validate that task can be archived
        if !task.can_transition_to(TaskState::Archived) {
            return Err(TaskError::InvalidStateTransition(task.state, TaskState::Archived));
        }
        
        task.state = TaskState::Archived;
        
        Ok(task.clone())
    }
    
    async fn health_check(&self) -> Result<()> {
        self.record_call("health_check");
        
        // Check for error injection
        self.check_error_injection()?;
        
        // Mock always reports healthy
        Ok(())
    }
    
    async fn get_stats(&self) -> Result<RepositoryStats> {
        self.record_call("get_stats");
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        let mut stats = RepositoryStats {
            total_tasks: tasks.len() as u64,
            ..Default::default()
        };
        
        // Count tasks by state
        for task in tasks.values() {
            *stats.tasks_by_state.entry(task.state).or_insert(0) += 1;
        }
        
        // Count tasks by owner
        for task in tasks.values() {
            if let Some(ref owner) = task.owner_agent_name {
                *stats.tasks_by_owner.entry(owner.clone()).or_insert(0) += 1;
            }
        }
        
        // Find latest timestamps
        stats.latest_created = tasks.values()
            .map(|t| t.inserted_at)
            .max();
        
        stats.latest_completed = tasks.values()
            .filter_map(|t| t.done_at)
            .max();
        
        Ok(stats)
    }

    // MCP v2 Advanced Multi-Agent Features

    async fn discover_work(&self, agent_name: &str, capabilities: &[String], max_tasks: u32) -> Result<Vec<Task>> {
        self.record_call_with_params("discover_work", &format!("agent={}, max_tasks={}", agent_name, max_tasks));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        let mut available_tasks: Vec<Task> = tasks.values()
            .filter(|task| {
                // Task is available for work (Created or InProgress without owner)
                matches!(task.state, TaskState::Created) || 
                (matches!(task.state, TaskState::InProgress) && task.owner_agent_name.is_none())
            })
            .filter(|task| {
                // Simple capability matching - if task has no required capabilities or agent has all required
                task.required_capabilities.is_empty() || 
                task.required_capabilities.iter().all(|cap| capabilities.contains(cap))
            })
            .cloned()
            .collect();
        
        // Sort by priority score (highest first)
        available_tasks.sort_by(|a, b| b.priority_score.partial_cmp(&a.priority_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Limit results
        available_tasks.truncate(max_tasks as usize);
        
        Ok(available_tasks)
    }

    async fn claim_task(&self, task_id: i32, agent_name: &str) -> Result<Task> {
        self.record_call_with_params("claim_task", &format!("task_id={}, agent={}", task_id, agent_name));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&task_id).ok_or_else(|| TaskError::NotFound(task_id.to_string()))?;
        
        // Check if already claimed
        if task.owner_agent_name.is_some() && task.owner_agent_name.as_deref() != Some(agent_name) {
            return Err(TaskError::AlreadyClaimed(task_id, task.owner_agent_name.clone().unwrap_or_default()));
        }
        
        // Claim the task
        task.owner_agent_name = Some(agent_name.to_string());
        
        Ok(task.clone())
    }

    async fn release_task(&self, task_id: i32, agent_name: &str) -> Result<Task> {
        self.record_call_with_params("release_task", &format!("task_id={}, agent={}", task_id, agent_name));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let mut tasks = self.tasks.lock();
        let task = tasks.get_mut(&task_id).ok_or_else(|| TaskError::NotFound(task_id.to_string()))?;
        
        // Check if agent owns the task
        if task.owner_agent_name.as_deref() != Some(agent_name) {
            return Err(TaskError::NotOwned(agent_name.to_string(), task_id));
        }
        
        // Release the task
        task.owner_agent_name = None;
        
        Ok(task.clone())
    }

    async fn start_work_session(&self, task_id: i32, agent_name: &str) -> Result<i32> {
        self.record_call_with_params("start_work_session", &format!("task_id={}, agent={}", task_id, agent_name));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        if !tasks.contains_key(&task_id) {
            return Err(TaskError::NotFound(task_id.to_string()));
        }
        
        // Return task_id as session_id for simplicity
        Ok(task_id)
    }

    async fn end_work_session(&self, session_id: i32, _notes: Option<String>, _productivity_score: Option<f64>) -> Result<()> {
        self.record_call_with_params("end_work_session", &format!("session_id={}", session_id));
        
        // Check for error injection
        self.check_error_injection()?;
        
        let tasks = self.tasks.lock();
        if !tasks.contains_key(&session_id) {
            return Err(TaskError::SessionNotFound(session_id));
        }
        
        Ok(())
    }
}