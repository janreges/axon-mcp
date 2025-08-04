//! Integration tests for MCP protocol
//!
//! Tests the full request/response cycle with mock repositories

use async_trait::async_trait;
use chrono::Utc;
use mcp_protocol::*;
use std::sync::Arc;
use task_core::error::{Result, TaskError};
use task_core::workspace_setup::WorkspaceContext;
use task_core::{
    NewTask, RepositoryStats, Task, TaskFilter, TaskMessage, TaskMessageRepository, TaskRepository,
    TaskState, UpdateTask, WorkspaceContextRepository,
};

/// Mock repository for testing
#[derive(Clone)]
struct MockRepository {
    tasks: Arc<tokio::sync::Mutex<Vec<Task>>>,
    next_id: Arc<tokio::sync::Mutex<i32>>,
}

impl MockRepository {
    fn new() -> Self {
        Self {
            tasks: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            next_id: Arc::new(tokio::sync::Mutex::new(1)),
        }
    }

    async fn get_next_id(&self) -> i32 {
        let mut id = self.next_id.lock().await;
        let current = *id;
        *id += 1;
        current
    }
}

#[async_trait]
impl TaskRepository for MockRepository {
    async fn create(&self, task: NewTask) -> Result<Task> {
        let mut tasks = self.tasks.lock().await;

        // Check for duplicate code
        if tasks.iter().any(|t| t.code == task.code) {
            return Err(TaskError::DuplicateCode(task.code));
        }

        let new_task = Task {
            id: self.get_next_id().await,
            code: task.code,
            name: task.name,
            description: task.description,
            owner_agent_name: task.owner_agent_name,
            state: TaskState::Created,
            inserted_at: Utc::now(),
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

        tasks.push(new_task.clone());
        Ok(new_task)
    }

    async fn update(&self, id: i32, updates: task_core::UpdateTask) -> Result<Task> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| TaskError::not_found_id(id))?;

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
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| TaskError::not_found_id(id))?;

        if !task.can_transition_to(state) {
            return Err(TaskError::invalid_transition(task.state, state));
        }

        task.state = state;
        if state == TaskState::Done {
            task.done_at = Some(Utc::now());
        }

        Ok(task.clone())
    }

    async fn get_by_id(&self, id: i32) -> Result<Option<Task>> {
        let tasks = self.tasks.lock().await;
        Ok(tasks.iter().find(|t| t.id == id).cloned())
    }

    async fn get_by_code(&self, code: &str) -> Result<Option<Task>> {
        let tasks = self.tasks.lock().await;
        Ok(tasks.iter().find(|t| t.code == code).cloned())
    }

    async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>> {
        let tasks = self.tasks.lock().await;
        let mut filtered: Vec<_> = tasks
            .iter()
            .filter(|task| {
                if let Some(ref owner) = filter.owner {
                    if task.owner_agent_name.as_deref() != Some(owner) {
                        return false;
                    }
                }
                if let Some(state) = filter.state {
                    if task.state != state {
                        return false;
                    }
                }
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

        filtered.sort_by_key(|t| t.id);

        // Apply pagination
        if let Some(offset) = filter.offset {
            if offset as usize >= filtered.len() {
                return Ok(Vec::new());
            }
            filtered = filtered.into_iter().skip(offset as usize).collect();
        }

        if let Some(limit) = filter.limit {
            filtered.truncate(limit as usize);
        }

        Ok(filtered)
    }

    async fn assign(&self, id: i32, new_owner: &str) -> Result<Task> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| TaskError::not_found_id(id))?;

        task.owner_agent_name = Some(new_owner.to_string());
        Ok(task.clone())
    }

    async fn archive(&self, id: i32) -> Result<Task> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| TaskError::not_found_id(id))?;

        if !task.can_transition_to(TaskState::Archived) {
            return Err(TaskError::invalid_transition(
                task.state,
                TaskState::Archived,
            ));
        }

        task.state = TaskState::Archived;
        Ok(task.clone())
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }

    async fn get_stats(&self) -> Result<RepositoryStats> {
        let tasks = self.tasks.lock().await;
        let mut stats = RepositoryStats {
            total_tasks: tasks.len() as u64,
            ..Default::default()
        };

        for task in tasks.iter() {
            *stats.tasks_by_state.entry(task.state).or_insert(0) += 1;
            if let Some(ref owner) = task.owner_agent_name {
                *stats.tasks_by_owner.entry(owner.clone()).or_insert(0) += 1;
            }
        }

        stats.latest_created = tasks.iter().map(|t| t.inserted_at).max();
        stats.latest_completed = tasks.iter().filter_map(|t| t.done_at).max();

        Ok(stats)
    }

    async fn discover_work(
        &self,
        _agent_name: &str,
        _capabilities: &[String],
        _max_tasks: u32,
    ) -> Result<Vec<Task>> {
        Ok(vec![])
    }

    async fn claim_task(&self, task_id: i32, agent_name: &str) -> Result<Task> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| TaskError::not_found_id(task_id))?;

        task.owner_agent_name = Some(agent_name.to_string());
        task.state = TaskState::InProgress;

        Ok(task.clone())
    }

    async fn release_task(&self, task_id: i32, _agent_name: &str) -> Result<Task> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| TaskError::not_found_id(task_id))?;

        task.owner_agent_name = None;
        task.state = TaskState::Created;

        Ok(task.clone())
    }

    async fn start_work_session(&self, task_id: i32, _agent_name: &str) -> Result<i32> {
        Ok(task_id * 100) // Mock session ID
    }

    async fn end_work_session(
        &self,
        _session_id: i32,
        _notes: Option<String>,
        _productivity_score: Option<f64>,
    ) -> Result<()> {
        Ok(())
    }

    async fn cleanup_timed_out_tasks(&self, _timeout_minutes: i64) -> Result<Vec<Task>> {
        Ok(vec![])
    }
}

/// Mock workspace context repository for testing
#[derive(Clone)]
struct MockWorkspaceContextRepository;

#[async_trait]
impl WorkspaceContextRepository for MockWorkspaceContextRepository {
    async fn create(&self, context: WorkspaceContext) -> Result<WorkspaceContext> {
        Ok(context)
    }

    async fn get_by_id(&self, _workspace_id: &str) -> Result<Option<WorkspaceContext>> {
        Ok(None)
    }

    async fn update(&self, context: WorkspaceContext) -> Result<WorkspaceContext> {
        Ok(context)
    }

    async fn delete(&self, _workspace_id: &str) -> Result<()> {
        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl TaskMessageRepository for MockRepository {
    async fn create_message(
        &self,
        task_code: &str,
        author_agent_name: &str,
        target_agent_name: Option<&str>,
        message_type: &str,
        content: &str,
        reply_to_message_id: Option<i32>,
    ) -> Result<TaskMessage> {
        Ok(TaskMessage {
            id: 1,
            task_code: task_code.to_string(),
            author_agent_name: author_agent_name.to_string(),
            target_agent_name: target_agent_name.map(|s| s.to_string()),
            message_type: message_type.to_string(),
            created_at: Utc::now(),
            content: content.to_string(),
            reply_to_message_id,
        })
    }

    async fn get_messages(
        &self,
        _task_code: &str,
        _author_agent_name: Option<&str>,
        _target_agent_name: Option<&str>,
        _message_type: Option<&str>,
        _reply_to_message_id: Option<i32>,
        _limit: Option<u32>,
    ) -> Result<Vec<TaskMessage>> {
        Ok(vec![])
    }

    async fn get_message_by_id(&self, _message_id: i32) -> Result<Option<TaskMessage>> {
        Ok(None)
    }
}

#[tokio::test]
async fn test_create_task_integration() {
    let repository = Arc::new(MockRepository::new());
    let workspace_repo = Arc::new(MockWorkspaceContextRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo, None);

    let params = CreateTaskParams {
        code: "TEST-001".to_string(),
        name: "Test Task".to_string(),
        description: "A test task".to_string(),
        owner_agent_name: Some("test-agent".to_string()),
        confidence_threshold: 0.8,
        estimated_effort: None,
        parent_task_id: None,
        required_capabilities: vec![],
        priority_score: 5.0,
        workflow_definition_id: None,
    };

    let result = handler.create_task(params).await;
    assert!(result.is_ok());

    let task = result.unwrap();
    assert_eq!(task.code, "TEST-001");
    assert_eq!(task.name, "Test Task");
    assert_eq!(task.state, TaskState::Created);
}

#[tokio::test]
async fn test_task_lifecycle_integration() {
    let repository = Arc::new(MockRepository::new());
    let workspace_repo = Arc::new(MockWorkspaceContextRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo, None);

    // Create task
    let create_params = CreateTaskParams {
        code: "LIFECYCLE-001".to_string(),
        name: "Lifecycle Test".to_string(),
        description: "Testing task lifecycle".to_string(),
        owner_agent_name: Some("test-agent".to_string()),
        confidence_threshold: 0.8,
        estimated_effort: None,
        parent_task_id: None,
        required_capabilities: vec![],
        priority_score: 5.0,
        workflow_definition_id: None,
    };

    let task = handler.create_task(create_params).await.unwrap();
    let task_id = task.id;

    // Update task
    let update_params = UpdateTaskParams {
        id: task_id,
        update_data: UpdateTask {
            name: Some("Updated Task".to_string()),
            description: None,
            owner_agent_name: None,
            workflow_definition_id: None,
            workflow_cursor: None,
            priority_score: Some(5.0),
            parent_task_id: None,
            required_capabilities: Some(vec![]),
            estimated_effort: None,
            confidence_threshold: Some(0.8),
        },
    };

    let updated_task = handler.update_task(update_params).await.unwrap();
    assert_eq!(updated_task.name, "Updated Task");

    // Set state to InProgress
    let state_params = SetStateParams {
        id: task_id,
        state: TaskState::InProgress,
    };

    let task_in_progress = handler.set_task_state(state_params).await.unwrap();
    assert_eq!(task_in_progress.state, TaskState::InProgress);

    // Set state to Done
    let done_params = SetStateParams {
        id: task_id,
        state: TaskState::Done,
    };

    let done_task = handler.set_task_state(done_params).await.unwrap();
    assert_eq!(done_task.state, TaskState::Done);
    assert!(done_task.done_at.is_some());

    // Archive task
    let archive_params = ArchiveTaskParams { id: task_id };
    let archived_task = handler.archive_task(archive_params).await.unwrap();
    assert_eq!(archived_task.state, TaskState::Archived);
}

#[tokio::test]
async fn test_error_handling_integration() {
    let repository = Arc::new(MockRepository::new());
    let workspace_repo = Arc::new(MockWorkspaceContextRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo, None);

    // Test duplicate code error
    let params1 = CreateTaskParams {
        code: "DUP-001".to_string(),
        name: "First Task".to_string(),
        description: "First task".to_string(),
        owner_agent_name: Some("agent1".to_string()),
        confidence_threshold: 0.8,
        estimated_effort: None,
        parent_task_id: None,
        required_capabilities: vec![],
        priority_score: 5.0,
        workflow_definition_id: None,
    };

    let params2 = CreateTaskParams {
        code: "DUP-001".to_string(), // Same code
        name: "Second Task".to_string(),
        description: "Second task".to_string(),
        owner_agent_name: Some("agent2".to_string()),
        confidence_threshold: 0.8,
        estimated_effort: None,
        parent_task_id: None,
        required_capabilities: vec![],
        priority_score: 5.0,
        workflow_definition_id: None,
    };

    handler.create_task(params1).await.unwrap();
    let duplicate_result = handler.create_task(params2).await;
    assert!(duplicate_result.is_err());
    assert!(matches!(
        duplicate_result.unwrap_err(),
        TaskError::DuplicateCode(_)
    ));

    // Test not found error
    let get_params = GetTaskByIdParams { id: 9999 };
    let not_found_result = handler.get_task_by_id(get_params).await.unwrap();
    assert!(not_found_result.is_none());

    // Test invalid state transition
    let create_params = CreateTaskParams {
        code: "INVALID-001".to_string(),
        name: "Invalid Transition Test".to_string(),
        description: "Testing invalid transitions".to_string(),
        owner_agent_name: Some("test-agent".to_string()),
        confidence_threshold: 0.8,
        estimated_effort: None,
        parent_task_id: None,
        required_capabilities: vec![],
        priority_score: 5.0,
        workflow_definition_id: None,
    };

    let task = handler.create_task(create_params).await.unwrap();

    // Try to go directly from Created to Done (invalid)
    let invalid_state_params = SetStateParams {
        id: task.id,
        state: TaskState::Done,
    };

    let invalid_result = handler.set_task_state(invalid_state_params).await;
    assert!(invalid_result.is_err());
    assert!(matches!(
        invalid_result.unwrap_err(),
        TaskError::InvalidStateTransition(_, _)
    ));
}

#[tokio::test]
async fn test_list_tasks_with_filters() {
    let repository = Arc::new(MockRepository::new());
    let workspace_repo = Arc::new(MockWorkspaceContextRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo, None);

    // Create multiple tasks
    for i in 1..=5 {
        let params = CreateTaskParams {
            code: format!("FILTER-{i:03}"),
            name: format!("Filter Test {i}"),
            description: "Filter test".to_string(),
            owner_agent_name: if i % 2 == 0 {
                Some("agent-even".to_string())
            } else {
                Some("agent-odd".to_string())
            },
            confidence_threshold: 0.8,
            estimated_effort: None,
            parent_task_id: None,
            required_capabilities: vec![],
            priority_score: 5.0,
            workflow_definition_id: None,
        };

        let task = handler.create_task(params).await.unwrap();

        // Set some tasks to InProgress
        if i <= 2 {
            let state_params = SetStateParams {
                id: task.id,
                state: TaskState::InProgress,
            };
            handler.set_task_state(state_params).await.unwrap();
        }
    }

    // Test filter by owner
    let list_params = ListTasksParams {
        owner: Some("agent-even".to_string()),
        ..Default::default()
    };

    let even_tasks = handler.list_tasks(list_params).await.unwrap();
    assert_eq!(even_tasks.len(), 2); // Tasks 2 and 4

    // Test filter by state
    let list_params = ListTasksParams {
        state: Some(TaskState::InProgress),
        ..Default::default()
    };

    let in_progress_tasks = handler.list_tasks(list_params).await.unwrap();
    assert_eq!(in_progress_tasks.len(), 2); // Tasks 1 and 2

    // Test limit
    let list_params = ListTasksParams {
        limit: Some(3),
        ..Default::default()
    };

    let limited_tasks = handler.list_tasks(list_params).await.unwrap();
    assert_eq!(limited_tasks.len(), 3);
}

#[tokio::test]
async fn test_serialization_integration() {
    let repository = Arc::new(MockRepository::new());
    let workspace_repo = Arc::new(MockWorkspaceContextRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo, None);

    let params = CreateTaskParams {
        code: "SERIAL-001".to_string(),
        name: "Serialization Test".to_string(),
        description: "Testing task serialization".to_string(),
        owner_agent_name: Some("serial-agent".to_string()),
        confidence_threshold: 0.8,
        estimated_effort: None,
        parent_task_id: None,
        required_capabilities: vec![],
        priority_score: 5.0,
        workflow_definition_id: None,
    };

    let task = handler.create_task(params).await.unwrap();

    // Test task serialization
    let serialized = serialize_task_for_mcp(&task).unwrap();
    assert_eq!(serialized["id"], task.id);
    assert_eq!(serialized["code"], "SERIAL-001");
    assert_eq!(serialized["name"], "Serialization Test");
    assert_eq!(serialized["state"], "Created");
    assert!(serialized["inserted_at"].is_string());
    assert!(serialized["done_at"].is_null());
}

#[tokio::test]
async fn test_health_check_integration() {
    let repository = Arc::new(MockRepository::new());
    let workspace_repo = Arc::new(MockWorkspaceContextRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo, None);

    let health = handler.health_check().await.unwrap();
    assert_eq!(health.status, "healthy");
    assert!(health.database);
    assert!(health.protocol);
    assert!(!health.version.is_empty());
}

#[tokio::test]
async fn test_assign_task_integration() {
    let repository = Arc::new(MockRepository::new());
    let workspace_repo = Arc::new(MockWorkspaceContextRepository);
    let handler = McpTaskHandler::new(repository.clone(), repository, workspace_repo, None);

    let create_params = CreateTaskParams {
        code: "ASSIGN-001".to_string(),
        name: "Assignment Test".to_string(),
        description: "Testing task assignment".to_string(),
        owner_agent_name: Some("original-agent".to_string()),
        confidence_threshold: 0.8,
        estimated_effort: None,
        parent_task_id: None,
        required_capabilities: vec![],
        priority_score: 5.0,
        workflow_definition_id: None,
    };

    let task = handler.create_task(create_params).await.unwrap();

    let assign_params = AssignTaskParams {
        id: task.id,
        new_owner: "new-agent".to_string(),
    };

    let assigned_task = handler.assign_task(assign_params).await.unwrap();
    assert_eq!(assigned_task.owner_agent_name.as_deref(), Some("new-agent"));
}
