# MOCKS01: Create Mock Repository Implementation

## Objective
Create a comprehensive mock implementation of the TaskRepository trait that provides in-memory storage and realistic behavior for testing all MCP v2 functionality.

## Implementation Details

### 1. Create Mock Repository Structure
In `mocks/src/repository.rs`:

```rust
use core::{
    error::{Result, TaskError},
    models::*,
    repository::*,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc, Duration};
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicI32, Ordering};

/// In-memory mock implementation of TaskRepository
#[derive(Clone)]
pub struct MockTaskRepository {
    // ID generators
    task_id_counter: Arc<AtomicI32>,
    message_id_counter: Arc<AtomicI32>,
    knowledge_id_counter: Arc<AtomicI32>,
    handoff_id_counter: Arc<AtomicI32>,
    help_id_counter: Arc<AtomicI32>,
    workflow_id_counter: Arc<AtomicI32>,
    event_id_counter: Arc<AtomicI32>,
    
    // Storage
    tasks: Arc<Mutex<HashMap<String, Task>>>,
    task_messages: Arc<Mutex<Vec<TaskMessage>>>,
    knowledge_objects: Arc<Mutex<Vec<KnowledgeObject>>>,
    agents: Arc<Mutex<HashMap<String, AgentProfile>>>,
    handoff_packages: Arc<Mutex<HashMap<i32, HandoffPackage>>>,
    help_requests: Arc<Mutex<HashMap<i32, HelpRequest>>>,
    workflow_definitions: Arc<Mutex<HashMap<i32, WorkflowDefinition>>>,
    system_events: Arc<Mutex<Vec<SystemEvent>>>,
    
    // Behavior configuration
    failure_probability: Arc<Mutex<f64>>,
    latency_ms: Arc<Mutex<u64>>,
}

impl MockTaskRepository {
    pub fn new() -> Self {
        Self {
            task_id_counter: Arc::new(AtomicI32::new(1)),
            message_id_counter: Arc::new(AtomicI32::new(1)),
            knowledge_id_counter: Arc::new(AtomicI32::new(1)),
            handoff_id_counter: Arc::new(AtomicI32::new(1)),
            help_id_counter: Arc::new(AtomicI32::new(1)),
            workflow_id_counter: Arc::new(AtomicI32::new(1)),
            event_id_counter: Arc::new(AtomicI32::new(1)),
            
            tasks: Arc::new(Mutex::new(HashMap::new())),
            task_messages: Arc::new(Mutex::new(Vec::new())),
            knowledge_objects: Arc::new(Mutex::new(Vec::new())),
            agents: Arc::new(Mutex::new(HashMap::new())),
            handoff_packages: Arc::new(Mutex::new(HashMap::new())),
            help_requests: Arc::new(Mutex::new(HashMap::new())),
            workflow_definitions: Arc::new(Mutex::new(HashMap::new())),
            system_events: Arc::new(Mutex::new(Vec::new())),
            
            failure_probability: Arc::new(Mutex::new(0.0)),
            latency_ms: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Configure failure probability for testing error scenarios
    pub fn set_failure_probability(&self, probability: f64) {
        *self.failure_probability.lock().unwrap() = probability;
    }
    
    /// Configure simulated latency
    pub fn set_latency_ms(&self, ms: u64) {
        *self.latency_ms.lock().unwrap() = ms;
    }
    
    /// Clear all data
    pub fn clear(&self) {
        self.tasks.lock().unwrap().clear();
        self.task_messages.lock().unwrap().clear();
        self.knowledge_objects.lock().unwrap().clear();
        self.agents.lock().unwrap().clear();
        self.handoff_packages.lock().unwrap().clear();
        self.help_requests.lock().unwrap().clear();
        self.workflow_definitions.lock().unwrap().clear();
        self.system_events.lock().unwrap().clear();
    }
    
    /// Simulate latency and potential failure
    async fn simulate_operation(&self) -> Result<()> {
        // Simulate latency
        let latency = *self.latency_ms.lock().unwrap();
        if latency > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(latency)).await;
        }
        
        // Simulate failure
        let failure_prob = *self.failure_probability.lock().unwrap();
        if failure_prob > 0.0 && rand::random::<f64>() < failure_prob {
            return Err(TaskError::Database("Simulated database error".to_string()));
        }
        
        Ok(())
    }
}

#[async_trait]
impl TaskRepository for MockTaskRepository {
    // ===== Task Methods =====
    
    async fn create_task(&self, new_task: NewTask) -> Result<Task> {
        self.simulate_operation().await?;
        
        let id = self.task_id_counter.fetch_add(1, Ordering::SeqCst);
        
        let task = Task {
            id,
            code: new_task.code.clone(),
            name: new_task.name,
            description: new_task.description,
            state: TaskState::Created,
            owner_agent_name: new_task.owner_agent_name,
            priority_score: new_task.priority_score.unwrap_or(5),
            parent_task_id: new_task.parent_task_id,
            workflow_definition_id: new_task.workflow_definition_id,
            workflow_cursor: None,
            failure_count: 0,
            required_capabilities: new_task.required_capabilities,
            confidence_threshold: new_task.confidence_threshold,
            estimated_effort_minutes: new_task.estimated_effort_minutes,
            actual_effort_minutes: None,
            inserted_at: Utc::now(),
            done_at: None,
        };
        
        let mut tasks = self.tasks.lock().unwrap();
        if tasks.contains_key(&task.code) {
            return Err(TaskError::AlreadyExists(format!("Task {} already exists", task.code)));
        }
        
        tasks.insert(task.code.clone(), task.clone());
        Ok(task)
    }
    
    async fn update_task(&self, task: Task) -> Result<Task> {
        self.simulate_operation().await?;
        
        let mut tasks = self.tasks.lock().unwrap();
        if !tasks.contains_key(&task.code) {
            return Err(TaskError::NotFound(format!("Task {} not found", task.code)));
        }
        
        tasks.insert(task.code.clone(), task.clone());
        Ok(task)
    }
    
    async fn get_task_by_id(&self, id: i32) -> Result<Option<Task>> {
        self.simulate_operation().await?;
        
        let tasks = self.tasks.lock().unwrap();
        Ok(tasks.values().find(|t| t.id == id).cloned())
    }
    
    async fn get_task_by_code(&self, code: &str) -> Result<Option<Task>> {
        self.simulate_operation().await?;
        
        let tasks = self.tasks.lock().unwrap();
        Ok(tasks.get(code).cloned())
    }
    
    async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>> {
        self.simulate_operation().await?;
        
        let tasks = self.tasks.lock().unwrap();
        let mut results: Vec<Task> = tasks.values()
            .filter(|task| {
                // Apply filters
                if let Some(owner) = &filter.owner_agent_name {
                    if &task.owner_agent_name != owner {
                        return false;
                    }
                }
                
                if let Some(state) = &filter.state {
                    if &task.state != state {
                        return false;
                    }
                }
                
                if let Some(min_priority) = filter.priority_min {
                    if task.priority_score < min_priority {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // Sort by priority and creation time
        results.sort_by(|a, b| {
            b.priority_score.cmp(&a.priority_score)
                .then_with(|| a.inserted_at.cmp(&b.inserted_at))
        });
        
        // Apply pagination
        if let Some(offset) = filter.offset {
            results = results.into_iter().skip(offset as usize).collect();
        }
        
        if let Some(limit) = filter.limit {
            results.truncate(limit as usize);
        }
        
        Ok(results)
    }
    
    async fn delete_task(&self, code: &str) -> Result<()> {
        self.simulate_operation().await?;
        
        let mut tasks = self.tasks.lock().unwrap();
        if tasks.remove(code).is_none() {
            return Err(TaskError::NotFound(format!("Task {} not found", code)));
        }
        
        Ok(())
    }
    
    async fn set_task_state(&self, code: &str, state: TaskState) -> Result<Task> {
        self.simulate_operation().await?;
        
        let mut tasks = self.tasks.lock().unwrap();
        let task = tasks.get_mut(code)
            .ok_or_else(|| TaskError::NotFound(format!("Task {} not found", code)))?;
        
        task.state = state;
        
        if state == TaskState::Done {
            task.done_at = Some(Utc::now());
            if let Some(start) = task.inserted_at.timestamp() {
                let duration = (Utc::now() - task.inserted_at).num_minutes() as i32;
                task.actual_effort_minutes = Some(duration);
            }
        }
        
        Ok(task.clone())
    }
    
    async fn search_tasks(&self, query: &str) -> Result<Vec<Task>> {
        self.simulate_operation().await?;
        
        let query_lower = query.to_lowercase();
        let tasks = self.tasks.lock().unwrap();
        
        let results: Vec<Task> = tasks.values()
            .filter(|task| {
                task.name.to_lowercase().contains(&query_lower) ||
                task.description.to_lowercase().contains(&query_lower) ||
                task.code.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();
        
        Ok(results)
    }
    
    // ===== Task Message Methods =====
    
    async fn add_task_message(&self, message: NewTaskMessage) -> Result<TaskMessage> {
        self.simulate_operation().await?;
        
        let id = self.message_id_counter.fetch_add(1, Ordering::SeqCst);
        
        let task_message = TaskMessage {
            id,
            task_code: message.task_code,
            author_agent_name: message.author_agent_name,
            message_type: message.message_type,
            content: message.content,
            created_at: Utc::now(),
            reply_to_message_id: message.reply_to_message_id,
            read_by: vec![],
        };
        
        let mut messages = self.task_messages.lock().unwrap();
        messages.push(task_message.clone());
        
        Ok(task_message)
    }
    
    async fn get_task_messages(&self, filter: MessageFilter) -> Result<Vec<TaskMessage>> {
        self.simulate_operation().await?;
        
        let messages = self.task_messages.lock().unwrap();
        let mut results: Vec<TaskMessage> = messages.iter()
            .filter(|msg| {
                if let Some(task_code) = &filter.task_code {
                    if &msg.task_code != task_code {
                        return false;
                    }
                }
                
                if !filter.message_types.is_empty() {
                    if !filter.message_types.contains(&msg.message_type) {
                        return false;
                    }
                }
                
                if let Some(since) = filter.since {
                    if msg.created_at < since {
                        return false;
                    }
                }
                
                if let Some(author) = &filter.author_agent_name {
                    if &msg.author_agent_name != author {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // Sort by creation time
        results.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        
        // Apply pagination
        if let Some(offset) = filter.offset {
            results = results.into_iter().skip(offset as usize).collect();
        }
        
        if let Some(limit) = filter.limit {
            results.truncate(limit as usize);
        }
        
        Ok(results)
    }
    
    async fn search_task_messages(&self, query: MessageSearchQuery) -> Result<Vec<TaskMessage>> {
        self.simulate_operation().await?;
        
        let query_lower = query.query.to_lowercase();
        let messages = self.task_messages.lock().unwrap();
        
        let mut results: Vec<TaskMessage> = messages.iter()
            .filter(|msg| {
                // Content search
                if !msg.content.to_lowercase().contains(&query_lower) {
                    return false;
                }
                
                // Task code filter
                if let Some(task_codes) = &query.task_codes {
                    if !task_codes.contains(&msg.task_code) {
                        return false;
                    }
                }
                
                // Message type filter
                if !query.message_types.is_empty() {
                    if !query.message_types.contains(&msg.message_type) {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        if let Some(limit) = query.limit {
            results.truncate(limit as usize);
        }
        
        Ok(results)
    }
    
    async fn get_message_thread(&self, message_id: i32) -> Result<Vec<TaskMessage>> {
        self.simulate_operation().await?;
        
        let messages = self.task_messages.lock().unwrap();
        
        // Find the message
        let message = messages.iter()
            .find(|m| m.id == message_id)
            .ok_or_else(|| TaskError::NotFound(format!("Message {} not found", message_id)))?;
        
        // Build thread by following reply chain
        let mut thread = vec![message.clone()];
        let mut current_id = message.reply_to_message_id;
        
        while let Some(id) = current_id {
            if let Some(parent) = messages.iter().find(|m| m.id == id) {
                thread.insert(0, parent.clone());
                current_id = parent.reply_to_message_id;
            } else {
                break;
            }
        }
        
        // Add replies
        let mut replies: Vec<_> = messages.iter()
            .filter(|m| m.reply_to_message_id == Some(message_id))
            .cloned()
            .collect();
        
        thread.append(&mut replies);
        
        Ok(thread)
    }
    
    // ===== Knowledge Object Methods =====
    
    async fn create_knowledge_object(&self, knowledge: NewKnowledgeObject) -> Result<KnowledgeObject> {
        self.simulate_operation().await?;
        
        let id = self.knowledge_id_counter.fetch_add(1, Ordering::SeqCst);
        
        let knowledge_object = KnowledgeObject {
            id,
            task_code: knowledge.task_code,
            author_agent_name: knowledge.author_agent_name,
            knowledge_type: knowledge.knowledge_type,
            title: knowledge.title,
            body: knowledge.body,
            tags: knowledge.tags,
            visibility: knowledge.visibility,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            parent_knowledge_id: knowledge.parent_knowledge_id,
            confidence_score: knowledge.confidence_score,
            artifacts: knowledge.artifacts,
            version: 1,
            is_archived: false,
        };
        
        let mut objects = self.knowledge_objects.lock().unwrap();
        objects.push(knowledge_object.clone());
        
        Ok(knowledge_object)
    }
    
    async fn get_knowledge_objects(&self, filter: KnowledgeFilter) -> Result<Vec<KnowledgeObject>> {
        self.simulate_operation().await?;
        
        let objects = self.knowledge_objects.lock().unwrap();
        let mut results: Vec<KnowledgeObject> = objects.iter()
            .filter(|ko| {
                if ko.is_archived {
                    return false;
                }
                
                if let Some(task_code) = &filter.task_code {
                    if &ko.task_code != task_code {
                        return false;
                    }
                }
                
                if !filter.knowledge_types.is_empty() {
                    if !filter.knowledge_types.contains(&ko.knowledge_type) {
                        return false;
                    }
                }
                
                if let Some(author) = &filter.author_agent_name {
                    if &ko.author_agent_name != author {
                        return false;
                    }
                }
                
                if let Some(visibility) = &filter.visibility {
                    if &ko.visibility != visibility {
                        return false;
                    }
                }
                
                if !filter.tags.is_empty() {
                    let has_tag = filter.tags.iter()
                        .any(|tag| ko.tags.contains(tag));
                    if !has_tag {
                        return false;
                    }
                }
                
                if let Some(since) = filter.since {
                    if ko.created_at < since {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        if let Some(offset) = filter.offset {
            results = results.into_iter().skip(offset as usize).collect();
        }
        
        if let Some(limit) = filter.limit {
            results.truncate(limit as usize);
        }
        
        Ok(results)
    }
    
    async fn search_knowledge(&self, query: KnowledgeSearchQuery) -> Result<Vec<KnowledgeObject>> {
        self.simulate_operation().await?;
        
        let query_lower = query.query.to_lowercase();
        let objects = self.knowledge_objects.lock().unwrap();
        
        let mut results: Vec<KnowledgeObject> = objects.iter()
            .filter(|ko| {
                if ko.is_archived {
                    return false;
                }
                
                // Search in title and body
                let matches = ko.title.to_lowercase().contains(&query_lower) ||
                             ko.body.to_lowercase().contains(&query_lower);
                
                if !matches {
                    return false;
                }
                
                // Apply filters
                if let Some(task_codes) = &query.task_codes {
                    if !task_codes.contains(&ko.task_code) {
                        return false;
                    }
                }
                
                if !query.knowledge_types.is_empty() {
                    if !query.knowledge_types.contains(&ko.knowledge_type) {
                        return false;
                    }
                }
                
                if !query.tags.is_empty() {
                    let has_tag = query.tags.iter()
                        .any(|tag| ko.tags.contains(tag));
                    if !has_tag {
                        return false;
                    }
                }
                
                if let Some(visibility) = &query.visibility_filter {
                    if &ko.visibility != visibility {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // Sort by relevance (simplified - just by creation date)
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        if let Some(limit) = query.limit {
            results.truncate(limit as usize);
        }
        
        Ok(results)
    }
    
    // ===== Agent Methods =====
    
    async fn register_agent(&self, agent: NewAgentProfile) -> Result<AgentProfile> {
        self.simulate_operation().await?;
        
        let profile = AgentProfile {
            name: agent.name.clone(),
            capabilities: agent.capabilities,
            specializations: agent.specializations,
            status: AgentStatus::Idle,
            current_load: 0,
            max_concurrent_tasks: agent.max_concurrent_tasks,
            reputation_score: 1.0,
            last_heartbeat: Utc::now(),
            registered_at: Utc::now(),
            description: agent.description,
            preferred_task_types: agent.preferred_task_types,
            timezone: agent.timezone,
        };
        
        let mut agents = self.agents.lock().unwrap();
        if agents.contains_key(&profile.name) {
            return Err(TaskError::AlreadyExists(format!("Agent {} already exists", profile.name)));
        }
        
        agents.insert(profile.name.clone(), profile.clone());
        Ok(profile)
    }
    
    async fn get_agent(&self, name: &str) -> Result<Option<AgentProfile>> {
        self.simulate_operation().await?;
        
        let agents = self.agents.lock().unwrap();
        Ok(agents.get(name).cloned())
    }
    
    async fn list_agents(&self, filter: AgentFilter) -> Result<Vec<AgentProfile>> {
        self.simulate_operation().await?;
        
        let agents = self.agents.lock().unwrap();
        let mut results: Vec<AgentProfile> = agents.values()
            .filter(|agent| {
                if let Some(status) = &filter.status {
                    if &agent.status != status {
                        return false;
                    }
                }
                
                if !filter.capabilities.is_empty() {
                    let has_capability = filter.capabilities.iter()
                        .any(|cap| agent.capabilities.contains(cap));
                    if !has_capability {
                        return false;
                    }
                }
                
                if filter.available_only && !agent.is_available() {
                    return false;
                }
                
                if let Some(min_rep) = filter.min_reputation {
                    if agent.reputation_score < min_rep {
                        return false;
                    }
                }
                
                if let Some(max_load) = filter.max_load_percentage {
                    let load_pct = (agent.current_load as f64 / agent.max_concurrent_tasks as f64) * 100.0;
                    if load_pct > max_load {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        results.sort_by(|a, b| {
            b.reputation_score.partial_cmp(&a.reputation_score).unwrap()
                .then_with(|| a.current_load.cmp(&b.current_load))
        });
        
        if let Some(offset) = filter.offset {
            results = results.into_iter().skip(offset as usize).collect();
        }
        
        if let Some(limit) = filter.limit {
            results.truncate(limit as usize);
        }
        
        Ok(results)
    }
    
    async fn update_agent(&self, agent: AgentProfile) -> Result<AgentProfile> {
        self.simulate_operation().await?;
        
        let mut agents = self.agents.lock().unwrap();
        if !agents.contains_key(&agent.name) {
            return Err(TaskError::NotFound(format!("Agent {} not found", agent.name)));
        }
        
        agents.insert(agent.name.clone(), agent.clone());
        Ok(agent)
    }
    
    async fn update_agent_status(&self, name: &str, status: AgentStatus) -> Result<()> {
        self.simulate_operation().await?;
        
        let mut agents = self.agents.lock().unwrap();
        let agent = agents.get_mut(name)
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", name)))?;
        
        agent.status = status;
        Ok(())
    }
    
    async fn update_agent_heartbeat(
        &self,
        name: &str,
        current_load: i32,
        status: Option<AgentStatus>,
    ) -> Result<()> {
        self.simulate_operation().await?;
        
        let mut agents = self.agents.lock().unwrap();
        let agent = agents.get_mut(name)
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", name)))?;
        
        agent.last_heartbeat = Utc::now();
        agent.current_load = current_load;
        
        if let Some(new_status) = status {
            agent.status = new_status;
        }
        
        Ok(())
    }
    
    // ===== Workflow Methods =====
    
    async fn create_workflow_definition(&self, workflow: NewWorkflowDefinition) -> Result<WorkflowDefinition> {
        self.simulate_operation().await?;
        
        let id = self.workflow_id_counter.fetch_add(1, Ordering::SeqCst);
        
        let definition = WorkflowDefinition {
            id,
            name: workflow.name,
            description: workflow.description,
            created_by: workflow.created_by,
            created_at: Utc::now(),
            steps: workflow.steps,
            parallel_execution: workflow.parallel_execution,
            timeout_minutes: workflow.timeout_minutes,
            retry_policy: workflow.retry_policy,
            is_active: true,
        };
        
        let mut workflows = self.workflow_definitions.lock().unwrap();
        workflows.insert(id, definition.clone());
        
        Ok(definition)
    }
    
    async fn get_workflow_definition(&self, id: i32) -> Result<Option<WorkflowDefinition>> {
        self.simulate_operation().await?;
        
        let workflows = self.workflow_definitions.lock().unwrap();
        Ok(workflows.get(&id).cloned())
    }
    
    // ===== System Event Methods =====
    
    async fn log_event(&self, mut event: SystemEvent) -> Result<()> {
        self.simulate_operation().await?;
        
        event.id = self.event_id_counter.fetch_add(1, Ordering::SeqCst);
        
        let mut events = self.system_events.lock().unwrap();
        events.push(event);
        
        Ok(())
    }
    
    async fn get_events(&self, filter: EventFilter) -> Result<Vec<SystemEvent>> {
        self.simulate_operation().await?;
        
        let events = self.system_events.lock().unwrap();
        let mut results: Vec<SystemEvent> = events.iter()
            .filter(|event| {
                if let Some(event_type) = &filter.event_type {
                    if &event.event_type != event_type {
                        return false;
                    }
                }
                
                if let Some(actor_type) = &filter.actor_type {
                    if &event.actor_type != actor_type {
                        return false;
                    }
                }
                
                if let Some(actor_id) = &filter.actor_id {
                    if &event.actor_id != actor_id {
                        return false;
                    }
                }
                
                if let Some(task_code) = &filter.task_code {
                    if event.task_code.as_ref() != Some(task_code) {
                        return false;
                    }
                }
                
                if let Some(since) = filter.since {
                    if event.timestamp < since {
                        return false;
                    }
                }
                
                if let Some(until) = filter.until {
                    if event.timestamp > until {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if let Some(limit) = filter.limit {
            results.truncate(limit as usize);
        }
        
        Ok(results)
    }
    
    // Additional mock-specific helper methods would go here...
}
```

### 2. Create Mock Data Generators
In `mocks/src/generators.rs`:

```rust
use core::models::*;
use chrono::{DateTime, Utc, Duration};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

/// Generate realistic test data
pub struct MockDataGenerator;

impl MockDataGenerator {
    /// Generate a random task code
    pub fn task_code() -> String {
        let prefix = ["ARCH", "DB", "API", "UI", "TEST", "DOC"]
            .choose(&mut thread_rng())
            .unwrap();
        
        let number: u32 = thread_rng().gen_range(1..1000);
        format!("{}-{:03}", prefix, number)
    }
    
    /// Generate a random agent name
    pub fn agent_name() -> String {
        let adjectives = ["swift", "clever", "diligent", "expert", "senior"];
        let roles = ["developer", "architect", "analyst", "engineer", "specialist"];
        
        let adj = adjectives.choose(&mut thread_rng()).unwrap();
        let role = roles.choose(&mut thread_rng()).unwrap();
        
        format!("{}-{}", adj, role)
    }
    
    /// Generate random capabilities
    pub fn capabilities(count: usize) -> Vec<String> {
        let all_capabilities = vec![
            "rust", "python", "javascript", "typescript",
            "architecture", "database", "api", "frontend",
            "testing", "documentation", "devops", "security",
            "performance", "monitoring", "debugging", "review"
        ];
        
        let mut rng = thread_rng();
        let mut selected = Vec::new();
        
        for _ in 0..count.min(all_capabilities.len()) {
            loop {
                let cap = all_capabilities.choose(&mut rng).unwrap();
                if !selected.contains(&cap.to_string()) {
                    selected.push(cap.to_string());
                    break;
                }
            }
        }
        
        selected
    }
    
    /// Generate a sample task
    pub fn task() -> NewTask {
        NewTask {
            code: Self::task_code(),
            name: Self::task_name(),
            description: Self::task_description(),
            owner_agent_name: Self::agent_name(),
            priority_score: Some(thread_rng().gen_range(1..10)),
            required_capabilities: Some(serde_json::to_string(&Self::capabilities(2)).unwrap()),
            estimated_effort_minutes: Some(thread_rng().gen_range(30..480)),
            ..Default::default()
        }
    }
    
    /// Generate a sample agent profile
    pub fn agent() -> NewAgentProfile {
        let capabilities = Self::capabilities(thread_rng().gen_range(3..8));
        let specializations = capabilities.iter()
            .take(2)
            .cloned()
            .collect();
        
        NewAgentProfile {
            name: Self::agent_name(),
            capabilities,
            specializations,
            max_concurrent_tasks: thread_rng().gen_range(3..10),
            description: Some("A skilled agent ready to tackle complex tasks".to_string()),
            ..Default::default()
        }
    }
    
    /// Generate a sample message
    pub fn message(task_code: String, author: String) -> NewTaskMessage {
        let message_types = vec![
            MessageType::Comment,
            MessageType::Question,
            MessageType::Update,
            MessageType::Solution,
        ];
        
        let message_type = message_types.choose(&mut thread_rng()).unwrap().clone();
        
        NewTaskMessage {
            task_code,
            author_agent_name: author,
            message_type,
            content: Self::message_content(&message_type),
            reply_to_message_id: None,
        }
    }
    
    /// Generate a sample knowledge object
    pub fn knowledge(task_code: String, author: String) -> NewKnowledgeObject {
        let knowledge_types = vec![
            KnowledgeType::Documentation,
            KnowledgeType::Decision,
            KnowledgeType::Learning,
            KnowledgeType::Reference,
        ];
        
        let knowledge_type = knowledge_types.choose(&mut thread_rng()).unwrap().clone();
        
        NewKnowledgeObject {
            task_code,
            author_agent_name: author,
            knowledge_type,
            title: Self::knowledge_title(&knowledge_type),
            body: Self::knowledge_body(&knowledge_type),
            tags: vec!["test".to_string(), "mock".to_string()],
            visibility: Visibility::Team,
            confidence_score: Some(thread_rng().gen_range(0.5..1.0)),
            ..Default::default()
        }
    }
    
    // Helper methods
    
    fn task_name() -> String {
        let actions = ["Implement", "Fix", "Refactor", "Optimize", "Document"];
        let targets = ["authentication", "database layer", "API endpoints", "user interface", "test suite"];
        
        format!("{} {}", 
            actions.choose(&mut thread_rng()).unwrap(),
            targets.choose(&mut thread_rng()).unwrap()
        )
    }
    
    fn task_description() -> String {
        "This task involves analyzing the current implementation and making necessary improvements to ensure optimal performance and maintainability.".to_string()
    }
    
    fn message_content(message_type: &MessageType) -> String {
        match message_type {
            MessageType::Comment => "I've reviewed the implementation and it looks good overall.".to_string(),
            MessageType::Question => "Could you clarify the expected behavior in edge cases?".to_string(),
            MessageType::Update => "Progress update: Completed 60% of the implementation.".to_string(),
            MessageType::Solution => "I've found a solution using a more efficient algorithm.".to_string(),
            _ => "Generic message content for testing.".to_string(),
        }
    }
    
    fn knowledge_title(knowledge_type: &KnowledgeType) -> String {
        match knowledge_type {
            KnowledgeType::Documentation => "API Usage Guidelines",
            KnowledgeType::Decision => "Architecture Decision: Database Choice",
            KnowledgeType::Learning => "Lessons Learned: Performance Optimization",
            KnowledgeType::Reference => "External Library Documentation",
            _ => "Knowledge Object",
        }.to_string()
    }
    
    fn knowledge_body(knowledge_type: &KnowledgeType) -> String {
        match knowledge_type {
            KnowledgeType::Documentation => {
                "## Overview\nThis document describes the proper usage of our API...\n\n## Examples\n```rust\n// Example code here\n```"
            }
            KnowledgeType::Decision => {
                "## Context\nWe needed to choose a database solution...\n\n## Decision\nWe selected SQLite because...\n\n## Consequences\n- Pros: Simple, embedded\n- Cons: Limited concurrency"
            }
            KnowledgeType::Learning => {
                "## Issue\nPerformance degradation under load...\n\n## Root Cause\nN+1 query problem...\n\n## Solution\nImplemented query batching..."
            }
            KnowledgeType::Reference => {
                "## Library: tokio\n\n### Key Features\n- Async runtime\n- Zero-cost abstractions\n\n### Usage\nSee official docs at..."
            }
            _ => "Generic knowledge content for testing purposes.",
        }.to_string()
    }
}
```

### 3. Create Test Scenarios Builder
In `mocks/src/scenarios.rs`:

```rust
use crate::{MockTaskRepository, MockDataGenerator};
use core::models::*;

/// Build common test scenarios
pub struct TestScenarioBuilder {
    repo: MockTaskRepository,
}

impl TestScenarioBuilder {
    pub fn new(repo: MockTaskRepository) -> Self {
        Self { repo }
    }
    
    /// Create a basic multi-agent system
    pub async fn create_basic_system(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create agents
        let agents = vec![
            ("rust-architect", vec!["rust", "architecture", "design"]),
            ("frontend-dev", vec!["javascript", "typescript", "frontend", "react"]),
            ("backend-dev", vec!["rust", "api", "database", "performance"]),
            ("qa-engineer", vec!["testing", "automation", "quality"]),
            ("devops-engineer", vec!["devops", "deployment", "monitoring"]),
        ];
        
        for (name, capabilities) in agents {
            let agent = NewAgentProfile {
                name: name.to_string(),
                capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
                specializations: vec![capabilities[0].to_string()],
                max_concurrent_tasks: 5,
                description: Some(format!("Expert in {}", capabilities[0])),
                ..Default::default()
            };
            
            self.repo.register_agent(agent).await?;
        }
        
        // Create some tasks
        let tasks = vec![
            ("ARCH-001", "Design system architecture", vec!["architecture", "design"], "rust-architect"),
            ("API-001", "Implement REST endpoints", vec!["rust", "api"], "backend-dev"),
            ("UI-001", "Create dashboard interface", vec!["frontend", "react"], "frontend-dev"),
            ("TEST-001", "Write integration tests", vec!["testing"], "qa-engineer"),
            ("OPS-001", "Setup CI/CD pipeline", vec!["devops"], "devops-engineer"),
        ];
        
        for (code, name, caps, owner) in tasks {
            let task = NewTask {
                code: code.to_string(),
                name: name.to_string(),
                description: format!("Task to {}", name),
                owner_agent_name: owner.to_string(),
                required_capabilities: Some(serde_json::to_string(&caps)?),
                priority_score: Some(5),
                ..Default::default()
            };
            
            self.repo.create_task(task).await?;
        }
        
        Ok(())
    }
    
    /// Create a workflow scenario
    pub async fn create_workflow_scenario(&self) -> Result<i32, Box<dyn std::error::Error>> {
        // Create workflow definition
        let steps = vec![
            WorkflowStep {
                name: "Design API".to_string(),
                description: "Design the API interface".to_string(),
                required_capabilities: vec!["architecture".to_string(), "api".to_string()],
                estimated_minutes: Some(120),
                instructions: "Create OpenAPI specification".to_string(),
            },
            WorkflowStep {
                name: "Implement Backend".to_string(),
                description: "Implement the API backend".to_string(),
                required_capabilities: vec!["rust".to_string(), "api".to_string()],
                estimated_minutes: Some(480),
                instructions: "Implement all endpoints with proper error handling".to_string(),
            },
            WorkflowStep {
                name: "Test API".to_string(),
                description: "Write and run API tests".to_string(),
                required_capabilities: vec!["testing".to_string()],
                estimated_minutes: Some(240),
                instructions: "Write comprehensive integration tests".to_string(),
            },
        ];
        
        let workflow = NewWorkflowDefinition {
            name: "API Development Workflow".to_string(),
            description: "Standard workflow for API development".to_string(),
            created_by: "system".to_string(),
            steps,
            parallel_execution: false,
            ..Default::default()
        };
        
        let created = self.repo.create_workflow_definition(workflow).await?;
        Ok(created.id)
    }
    
    /// Create a help request scenario
    pub async fn create_help_scenario(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure we have tasks
        self.create_basic_system().await?;
        
        // Create help requests
        let help_request = NewHelpRequest {
            requesting_agent_name: "frontend-dev".to_string(),
            task_code: "UI-001".to_string(),
            help_type: HelpType::TechnicalQuestion,
            description: "How should I handle state management in the dashboard?".to_string(),
            urgency: Urgency::Medium,
            related_capabilities: vec!["react".to_string(), "frontend".to_string()],
        };
        
        self.repo.create_help_request(help_request).await?;
        
        // Create a blocker
        let blocker = NewHelpRequest {
            requesting_agent_name: "backend-dev".to_string(),
            task_code: "API-001".to_string(),
            help_type: HelpType::Blocker,
            description: "Database connection pool is not working correctly".to_string(),
            urgency: Urgency::High,
            related_capabilities: vec!["database".to_string(), "rust".to_string()],
        };
        
        self.repo.create_help_request(blocker).await?;
        
        Ok(())
    }
    
    /// Create a complex task hierarchy
    pub async fn create_task_hierarchy(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create parent task
        let parent = NewTask {
            code: "FEAT-001".to_string(),
            name: "Implement user authentication".to_string(),
            description: "Complete authentication system with OAuth support".to_string(),
            owner_agent_name: "rust-architect".to_string(),
            priority_score: Some(8),
            ..Default::default()
        };
        
        let parent_task = self.repo.create_task(parent).await?;
        
        // Create subtasks
        let subtasks = vec![
            ("FEAT-001-001", "Design auth architecture", "rust-architect"),
            ("FEAT-001-002", "Implement JWT tokens", "backend-dev"),
            ("FEAT-001-003", "Create login UI", "frontend-dev"),
            ("FEAT-001-004", "Add OAuth providers", "backend-dev"),
            ("FEAT-001-005", "Test auth flows", "qa-engineer"),
        ];
        
        for (code, name, owner) in subtasks {
            let task = NewTask {
                code: code.to_string(),
                name: name.to_string(),
                description: format!("Subtask: {}", name),
                owner_agent_name: owner.to_string(),
                parent_task_id: Some(parent_task.id),
                priority_score: Some(7),
                ..Default::default()
            };
            
            self.repo.create_task(task).await?;
        }
        
        Ok(())
    }
}
```

### 4. Create Contract Tests
In `mocks/src/contract_tests.rs`:

```rust
/// Contract tests that all TaskRepository implementations must pass
#[cfg(test)]
mod contract_tests {
    use super::*;
    use core::{models::*, repository::TaskRepository};
    
    /// Test suite for TaskRepository implementations
    pub async fn run_repository_contract_tests<R: TaskRepository>(repo: R) {
        test_task_crud(&repo).await;
        test_task_state_transitions(&repo).await;
        test_message_operations(&repo).await;
        test_knowledge_operations(&repo).await;
        test_agent_operations(&repo).await;
        test_workflow_operations(&repo).await;
        test_help_request_operations(&repo).await;
        test_handoff_operations(&repo).await;
    }
    
    async fn test_task_crud<R: TaskRepository>(repo: &R) {
        // Create
        let new_task = NewTask {
            code: "TEST-001".to_string(),
            name: "Test task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            ..Default::default()
        };
        
        let created = repo.create_task(new_task).await
            .expect("Should create task");
        
        assert_eq!(created.code, "TEST-001");
        assert_eq!(created.state, TaskState::Created);
        
        // Read
        let fetched = repo.get_task_by_code("TEST-001").await
            .expect("Should fetch task")
            .expect("Task should exist");
        
        assert_eq!(fetched.id, created.id);
        
        // Update
        let mut updated = fetched.clone();
        updated.name = "Updated name".to_string();
        
        let saved = repo.update_task(updated).await
            .expect("Should update task");
        
        assert_eq!(saved.name, "Updated name");
        
        // Delete
        repo.delete_task("TEST-001").await
            .expect("Should delete task");
        
        let deleted = repo.get_task_by_code("TEST-001").await
            .expect("Should not error");
        
        assert!(deleted.is_none());
    }
    
    async fn test_task_state_transitions<R: TaskRepository>(repo: &R) {
        let task = NewTask {
            code: "STATE-001".to_string(),
            name: "State test".to_string(),
            description: "Test state transitions".to_string(),
            owner_agent_name: "test-agent".to_string(),
            ..Default::default()
        };
        
        let created = repo.create_task(task).await.unwrap();
        assert_eq!(created.state, TaskState::Created);
        
        // Transition to InProgress
        let in_progress = repo.set_task_state("STATE-001", TaskState::InProgress).await.unwrap();
        assert_eq!(in_progress.state, TaskState::InProgress);
        
        // Transition to Done
        let done = repo.set_task_state("STATE-001", TaskState::Done).await.unwrap();
        assert_eq!(done.state, TaskState::Done);
        assert!(done.done_at.is_some());
    }
    
    // Additional contract tests...
}
```

## Files to Create
- `mocks/src/repository.rs` - Mock repository implementation
- `mocks/src/generators.rs` - Test data generators
- `mocks/src/scenarios.rs` - Test scenario builders
- `mocks/src/contract_tests.rs` - Contract test suite
- `mocks/src/lib.rs` - Module exports

## Testing Requirements
1. All repository methods must be implemented
2. Contract tests must pass
3. Concurrent access must be thread-safe
4. Failure simulation must work correctly
5. Test data generators must produce valid data
6. Scenarios must create consistent state

## Usage Example
```rust
use mocks::{MockTaskRepository, TestScenarioBuilder};

#[tokio::test]
async fn test_with_mock() {
    let repo = MockTaskRepository::new();
    
    // Create test scenario
    let builder = TestScenarioBuilder::new(repo.clone());
    builder.create_basic_system().await.unwrap();
    
    // Test with simulated failures
    repo.set_failure_probability(0.1); // 10% failure rate
    repo.set_latency_ms(50); // 50ms latency
    
    // Run tests...
}
```

## Notes
- Thread-safe in-memory storage
- Configurable failure simulation
- Realistic test data generation
- Pre-built test scenarios
- Contract test suite for verification