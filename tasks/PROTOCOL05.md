# PROTOCOL05: Implement Workflow and Handoff Handlers

## Objective
Implement all workflow and handoff-related protocol handlers, enabling task decomposition, workflow orchestration, and seamless handoffs between agents.

## Implementation Details

### 1. Extend Protocol Handler with Workflow Methods
In `mcp-protocol/src/handler.rs`, add workflow and handoff implementations:

```rust
// Add to the existing McpProtocolHandler implementation
impl<R: TaskRepository> McpProtocolHandler<R> {
    // ... existing methods ...
    
    // ===== Workflow Methods =====
    
    async fn handle_create_workflow(&self, params: CreateWorkflowParams) -> Result<WorkflowDefinition> {
        // Validate workflow structure
        params.validate()?;
        
        // Validate all capabilities exist
        for step in &params.steps {
            for capability in &step.required_capabilities {
                let agents = self.repository
                    .find_agents_by_capability(capability, 1)
                    .await?;
                
                if agents.is_empty() {
                    return Err(TaskError::Validation(
                        format!("No agents available with capability: {}", capability)
                    ));
                }
            }
        }
        
        let workflow = NewWorkflowDefinition {
            name: params.name,
            description: params.description,
            created_by: params.created_by,
            steps: params.steps,
            parallel_execution: params.parallel_execution.unwrap_or(false),
            timeout_minutes: params.timeout_minutes,
            retry_policy: params.retry_policy,
        };
        
        let created = self.repository.create_workflow_definition(workflow).await?;
        
        // Log creation event
        self.log_workflow_event("workflow_created", &created).await?;
        
        Ok(created)
    }
    
    async fn handle_assign_workflow(&self, params: AssignWorkflowParams) -> Result<Task> {
        // Get workflow definition
        let workflow = self.repository
            .get_workflow_definition(params.workflow_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Workflow {} not found", params.workflow_id)
            ))?;
        
        // Create task with workflow
        let new_task = NewTask {
            code: params.task_code,
            name: params.task_name,
            description: format!("{}\n\nWorkflow: {}", params.task_description, workflow.name),
            owner_agent_name: params.initial_agent.clone().unwrap_or("workflow-engine".to_string()),
            workflow_definition_id: Some(params.workflow_id),
            required_capabilities: Some(serde_json::to_string(&workflow.steps[0].required_capabilities).unwrap()),
            parent_task_id: params.parent_task_id,
        };
        
        let task = self.repository.create_task(new_task).await?;
        
        // If initial agent specified, assign it
        if let Some(agent_name) = params.initial_agent {
            self.repository.update_task_owner(&task.code, &agent_name).await?;
        }
        
        // Log workflow assignment
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "workflow_assigned".to_string(),
            actor_type: ActorType::System,
            actor_id: "workflow-engine".to_string(),
            task_code: Some(task.code.clone()),
            payload: serde_json::json!({
                "workflow_id": params.workflow_id,
                "workflow_name": workflow.name,
                "total_steps": workflow.steps.len(),
            }),
            correlation_id: Some(format!("workflow-{}", params.workflow_id)),
        };
        
        self.repository.log_event(event).await?;
        
        Ok(task)
    }
    
    async fn handle_advance_workflow(&self, params: AdvanceWorkflowParams) -> Result<WorkflowAdvanceResult> {
        // Get task
        let mut task = self.repository
            .get_task_by_code(&params.task_code)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Task {} not found", params.task_code)
            ))?;
        
        // Verify task has workflow
        let workflow_id = task.workflow_definition_id
            .ok_or_else(|| TaskError::Validation("Task has no workflow assigned".to_string()))?;
        
        let workflow = self.repository
            .get_workflow_definition(workflow_id)
            .await?
            .ok_or_else(|| TaskError::NotFound("Workflow definition not found".to_string()))?;
        
        // Get current position
        let current_step = task.workflow_cursor.unwrap_or(0) as usize;
        
        // Check if already at end
        if current_step >= workflow.steps.len() {
            return Ok(WorkflowAdvanceResult {
                task_code: task.code,
                current_step: current_step as i32,
                next_step: None,
                workflow_complete: true,
                next_agent: None,
                handoff_created: false,
            });
        }
        
        // Advance to next step
        let next_step = current_step + 1;
        let workflow_complete = next_step >= workflow.steps.len();
        
        // Update task workflow cursor
        task.workflow_cursor = Some(next_step as i32);
        self.repository.update_task(task.clone()).await?;
        
        let mut result = WorkflowAdvanceResult {
            task_code: task.code.clone(),
            current_step: current_step as i32,
            next_step: if workflow_complete { None } else { Some(next_step as i32) },
            workflow_complete,
            next_agent: None,
            handoff_created: false,
        };
        
        // If not complete, prepare for next step
        if !workflow_complete {
            let next_workflow_step = &workflow.steps[next_step];
            
            // Find agent for next step
            if let Some(agent_name) = params.next_agent {
                // Verify agent has required capabilities
                let agent = self.repository.get_agent(&agent_name).await?
                    .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", agent_name)))?;
                
                let has_capabilities = next_workflow_step.required_capabilities.iter()
                    .all(|cap| agent.has_capability(cap));
                
                if !has_capabilities {
                    return Err(TaskError::Validation(
                        "Next agent doesn't have required capabilities for workflow step".to_string()
                    ));
                }
                
                result.next_agent = Some(agent_name.clone());
                
                // Create handoff
                let handoff = self.create_workflow_handoff(
                    &task,
                    &params.completed_by,
                    &agent_name,
                    next_workflow_step,
                ).await?;
                
                result.handoff_created = true;
            } else {
                // Auto-discover next agent
                if let Some(agent_rec) = self.find_agent_for_workflow_step(next_workflow_step).await? {
                    result.next_agent = Some(agent_rec.agent_name.clone());
                    
                    // Create handoff
                    let handoff = self.create_workflow_handoff(
                        &task,
                        &params.completed_by,
                        &agent_rec.agent_name,
                        next_workflow_step,
                    ).await?;
                    
                    result.handoff_created = true;
                }
            }
        } else {
            // Workflow complete, update task state
            self.repository.set_task_state(&task.code, TaskState::Review).await?;
        }
        
        // Log advancement
        self.log_workflow_advancement(&task, current_step, &result).await?;
        
        Ok(result)
    }
    
    // ===== Handoff Methods =====
    
    async fn handle_create_handoff(&self, params: CreateHandoffParams) -> Result<HandoffPackage> {
        // Validate task exists and is owned by from_agent
        let task = self.repository
            .get_task_by_code(&params.task_code)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Task {} not found", params.task_code)))?;
        
        if task.owner_agent_name != params.from_agent_name {
            return Err(TaskError::Validation(
                "Agent can only handoff tasks they own".to_string()
            ));
        }
        
        // Validate to_agent exists if specified
        if let Some(to_agent) = &params.to_agent_name {
            let agent_exists = self.repository.get_agent(to_agent).await?.is_some();
            if !agent_exists {
                return Err(TaskError::NotFound(format!("Agent {} not found", to_agent)));
            }
        }
        
        // Create knowledge snapshot
        let knowledge_snapshot = self.create_knowledge_snapshot(&params.task_code).await?;
        
        let handoff = NewHandoffPackage {
            task_code: params.task_code.clone(),
            from_agent_name: params.from_agent_name,
            to_agent_name: params.to_agent_name,
            to_capability: params.to_capability,
            context: params.context,
            confidence_score: params.confidence_score.unwrap_or(0.8),
            knowledge_snapshot,
            recommended_next_steps: params.recommended_next_steps,
            estimated_effort: params.estimated_effort,
        };
        
        let created = self.repository.create_handoff_package(handoff).await?;
        
        // Update task state to PendingHandoff
        self.repository.set_task_state(&params.task_code, TaskState::PendingHandoff).await?;
        
        // Log handoff creation
        self.log_handoff_event("handoff_created", &created).await?;
        
        // Send notifications if to_agent specified
        if let Some(to_agent) = &created.to_agent_name {
            self.notify_handoff_recipient(to_agent, &created).await?;
        }
        
        Ok(created)
    }
    
    async fn handle_accept_handoff(&self, params: AcceptHandoffParams) -> Result<Task> {
        // Get handoff package
        let mut handoff = self.repository
            .get_handoff_package(params.handoff_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Handoff {} not found", params.handoff_id)
            ))?;
        
        // Verify it's not already accepted
        if handoff.accepted_at.is_some() {
            return Err(TaskError::Validation("Handoff already accepted".to_string()));
        }
        
        // Verify accepting agent matches to_agent or has capability
        if let Some(to_agent) = &handoff.to_agent_name {
            if to_agent != &params.accepting_agent_name {
                return Err(TaskError::Validation(
                    "Handoff is designated for a different agent".to_string()
                ));
            }
        } else if let Some(to_capability) = &handoff.to_capability {
            // Verify agent has the capability
            let agent = self.repository
                .get_agent(&params.accepting_agent_name)
                .await?
                .ok_or_else(|| TaskError::NotFound(
                    format!("Agent {} not found", params.accepting_agent_name)
                ))?;
            
            if !agent.has_capability(to_capability) {
                return Err(TaskError::Validation(
                    format!("Agent doesn't have required capability: {}", to_capability)
                ));
            }
        }
        
        // Accept handoff
        handoff.accepted_at = Some(Utc::now());
        handoff.accepted_by = Some(params.accepting_agent_name.clone());
        self.repository.update_handoff_package(handoff.clone()).await?;
        
        // Transfer task ownership
        let mut task = self.repository
            .get_task_by_code(&handoff.task_code)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Task {} not found", handoff.task_code)
            ))?;
        
        task.owner_agent_name = params.accepting_agent_name.clone();
        task.state = TaskState::InProgress;
        let updated_task = self.repository.update_task(task).await?;
        
        // Import knowledge snapshot if requested
        if params.import_knowledge.unwrap_or(true) {
            self.import_knowledge_snapshot(
                &handoff.task_code,
                &params.accepting_agent_name,
                &handoff.knowledge_snapshot,
            ).await?;
        }
        
        // Log acceptance
        self.log_handoff_event("handoff_accepted", &handoff).await?;
        
        Ok(updated_task)
    }
    
    async fn handle_reject_handoff(&self, params: RejectHandoffParams) -> Result<()> {
        // Get handoff package
        let mut handoff = self.repository
            .get_handoff_package(params.handoff_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Handoff {} not found", params.handoff_id)
            ))?;
        
        // Verify it's not already accepted/rejected
        if handoff.accepted_at.is_some() || handoff.rejected_at.is_some() {
            return Err(TaskError::Validation("Handoff already processed".to_string()));
        }
        
        // Reject handoff
        handoff.rejected_at = Some(Utc::now());
        handoff.rejected_by = Some(params.rejecting_agent_name.clone());
        handoff.rejection_reason = params.reason;
        self.repository.update_handoff_package(handoff.clone()).await?;
        
        // Revert task state if still PendingHandoff
        let task = self.repository
            .get_task_by_code(&handoff.task_code)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Task {} not found", handoff.task_code)
            ))?;
        
        if task.state == TaskState::PendingHandoff {
            self.repository.set_task_state(&handoff.task_code, TaskState::InProgress).await?;
        }
        
        // Log rejection
        self.log_handoff_event("handoff_rejected", &handoff).await?;
        
        // Notify original agent
        self.notify_handoff_rejection(&handoff.from_agent_name, &handoff).await?;
        
        Ok(())
    }
    
    // ===== Task Decomposition =====
    
    async fn handle_decompose_task(&self, params: DecomposeTaskParams) -> Result<Vec<Task>> {
        // Get parent task
        let parent_task = self.repository
            .get_task_by_code(&params.parent_task_code)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Task {} not found", params.parent_task_code)
            ))?;
        
        // Validate ownership
        if parent_task.owner_agent_name != params.decomposing_agent_name {
            return Err(TaskError::Validation(
                "Only task owner can decompose task".to_string()
            ));
        }
        
        let mut created_tasks = Vec::new();
        
        // Create subtasks
        for (index, subtask) in params.subtasks.into_iter().enumerate() {
            let code = format!("{}-{:03}", params.parent_task_code, index + 1);
            
            let new_task = NewTask {
                code: code.clone(),
                name: subtask.name,
                description: subtask.description,
                owner_agent_name: subtask.assigned_agent.unwrap_or("unassigned".to_string()),
                parent_task_id: Some(parent_task.id),
                required_capabilities: subtask.required_capabilities
                    .map(|caps| serde_json::to_string(&caps).unwrap()),
                priority_score: subtask.priority.unwrap_or(parent_task.priority_score),
                estimated_effort_minutes: subtask.estimated_effort_minutes,
            };
            
            let created = self.repository.create_task(new_task).await?;
            created_tasks.push(created);
        }
        
        // Update parent task state if requested
        if params.parent_state_after.is_some() {
            let new_state = match params.parent_state_after.as_ref().unwrap().as_str() {
                "waiting" => TaskState::Waiting,
                "blocked" => TaskState::Blocked,
                _ => TaskState::InProgress,
            };
            
            self.repository.set_task_state(&params.parent_task_code, new_state).await?;
        }
        
        // Log decomposition
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "task_decomposed".to_string(),
            actor_type: ActorType::Agent,
            actor_id: params.decomposing_agent_name,
            task_code: Some(params.parent_task_code.clone()),
            payload: serde_json::json!({
                "subtask_count": created_tasks.len(),
                "subtask_codes": created_tasks.iter().map(|t| &t.code).collect::<Vec<_>>(),
            }),
            correlation_id: Some(format!("decompose-{}", params.parent_task_code)),
        };
        
        self.repository.log_event(event).await?;
        
        Ok(created_tasks)
    }
    
    // Helper methods
    
    async fn create_workflow_handoff(
        &self,
        task: &Task,
        from_agent: &str,
        to_agent: &str,
        workflow_step: &WorkflowStep,
    ) -> Result<HandoffPackage> {
        let handoff = NewHandoffPackage {
            task_code: task.code.clone(),
            from_agent_name: from_agent.to_string(),
            to_agent_name: Some(to_agent.to_string()),
            to_capability: None,
            context: format!(
                "Workflow step {}: {}",
                task.workflow_cursor.unwrap_or(0),
                workflow_step.description
            ),
            confidence_score: 0.9,
            knowledge_snapshot: self.create_knowledge_snapshot(&task.code).await?,
            recommended_next_steps: Some(workflow_step.instructions.clone()),
            estimated_effort: workflow_step.estimated_minutes,
        };
        
        self.repository.create_handoff_package(handoff).await
    }
    
    async fn find_agent_for_workflow_step(
        &self,
        step: &WorkflowStep,
    ) -> Result<Option<AgentRecommendation>> {
        // Use agent discovery service
        let discovery = AgentDiscoveryService::new(Arc::clone(&self.repository));
        
        // Create a mock task with the step's requirements
        let mock_task = Task {
            id: 0,
            code: "workflow-step".to_string(),
            name: step.name.clone(),
            description: step.description.clone(),
            required_capabilities: Some(serde_json::to_string(&step.required_capabilities).unwrap()),
            ..Default::default()
        };
        
        discovery.find_best_agent_for_task(&mock_task).await
    }
    
    async fn create_knowledge_snapshot(&self, task_code: &str) -> Result<serde_json::Value> {
        // Get recent knowledge objects for the task
        let filter = KnowledgeFilter {
            task_code: Some(task_code.to_string()),
            visibility: Some(Visibility::Public),
            limit: Some(20),
            ..Default::default()
        };
        
        let knowledge_objects = self.repository.get_knowledge_objects(filter).await?;
        
        // Get recent messages
        let message_filter = MessageFilter {
            task_code: Some(task_code.to_string()),
            limit: Some(20),
            ..Default::default()
        };
        
        let messages = self.repository.get_task_messages(message_filter).await?;
        
        Ok(serde_json::json!({
            "knowledge_objects": knowledge_objects,
            "recent_messages": messages,
            "snapshot_timestamp": Utc::now().to_rfc3339(),
        }))
    }
    
    async fn import_knowledge_snapshot(
        &self,
        task_code: &str,
        agent_name: &str,
        snapshot: &serde_json::Value,
    ) -> Result<()> {
        // This would parse the snapshot and create new knowledge objects
        // marked as imported from handoff
        
        if let Some(knowledge_array) = snapshot.get("knowledge_objects").and_then(|v| v.as_array()) {
            for knowledge_json in knowledge_array {
                if let Ok(mut knowledge) = serde_json::from_value::<KnowledgeObject>(knowledge_json.clone()) {
                    // Create as imported knowledge
                    let new_knowledge = NewKnowledgeObject {
                        task_code: task_code.to_string(),
                        author_agent_name: agent_name.to_string(),
                        knowledge_type: KnowledgeType::Reference,
                        title: format!("[Imported] {}", knowledge.title),
                        body: knowledge.body,
                        tags: vec!["imported".to_string(), "handoff".to_string()],
                        visibility: Visibility::Team,
                        parent_knowledge_id: None,
                        confidence_score: Some(knowledge.confidence_score.unwrap_or(0.8) * 0.9),
                        artifacts: Some(serde_json::json!({
                            "original_author": knowledge.author_agent_name,
                            "import_timestamp": Utc::now().to_rfc3339(),
                        })),
                    };
                    
                    let _ = self.repository.create_knowledge_object(new_knowledge).await;
                }
            }
        }
        
        Ok(())
    }
    
    async fn log_workflow_event(&self, event_type: &str, workflow: &WorkflowDefinition) -> Result<()> {
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            actor_type: ActorType::System,
            actor_id: "workflow-engine".to_string(),
            task_code: None,
            payload: serde_json::json!({
                "workflow_id": workflow.id,
                "workflow_name": workflow.name,
                "steps": workflow.steps.len(),
            }),
            correlation_id: Some(format!("workflow-{}", workflow.id)),
        };
        
        self.repository.log_event(event).await
    }
    
    async fn log_workflow_advancement(
        &self,
        task: &Task,
        from_step: usize,
        result: &WorkflowAdvanceResult,
    ) -> Result<()> {
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "workflow_advanced".to_string(),
            actor_type: ActorType::System,
            actor_id: "workflow-engine".to_string(),
            task_code: Some(task.code.clone()),
            payload: serde_json::json!({
                "from_step": from_step,
                "to_step": result.next_step,
                "workflow_complete": result.workflow_complete,
                "next_agent": result.next_agent,
            }),
            correlation_id: task.workflow_definition_id
                .map(|id| format!("workflow-{}", id)),
        };
        
        self.repository.log_event(event).await
    }
    
    async fn log_handoff_event(&self, event_type: &str, handoff: &HandoffPackage) -> Result<()> {
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            actor_type: ActorType::Agent,
            actor_id: match event_type {
                "handoff_created" => handoff.from_agent_name.clone(),
                "handoff_accepted" => handoff.accepted_by.clone().unwrap_or_default(),
                "handoff_rejected" => handoff.rejected_by.clone().unwrap_or_default(),
                _ => "system".to_string(),
            },
            task_code: Some(handoff.task_code.clone()),
            payload: serde_json::json!({
                "handoff_id": handoff.id,
                "from_agent": handoff.from_agent_name,
                "to_agent": handoff.to_agent_name,
                "to_capability": handoff.to_capability,
                "confidence_score": handoff.confidence_score,
            }),
            correlation_id: Some(format!("handoff-{}", handoff.id)),
        };
        
        self.repository.log_event(event).await
    }
    
    async fn notify_handoff_recipient(&self, agent_name: &str, handoff: &HandoffPackage) -> Result<()> {
        // This would integrate with the notification system
        // For now, just log an event
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "handoff_notification_sent".to_string(),
            actor_type: ActorType::System,
            actor_id: "notification-system".to_string(),
            task_code: Some(handoff.task_code.clone()),
            payload: serde_json::json!({
                "recipient": agent_name,
                "handoff_id": handoff.id,
            }),
            correlation_id: Some(format!("handoff-{}", handoff.id)),
        };
        
        self.repository.log_event(event).await
    }
    
    async fn notify_handoff_rejection(&self, agent_name: &str, handoff: &HandoffPackage) -> Result<()> {
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "handoff_rejection_notification".to_string(),
            actor_type: ActorType::System,
            actor_id: "notification-system".to_string(),
            task_code: Some(handoff.task_code.clone()),
            payload: serde_json::json!({
                "recipient": agent_name,
                "handoff_id": handoff.id,
                "rejected_by": handoff.rejected_by,
                "reason": handoff.rejection_reason,
            }),
            correlation_id: Some(format!("handoff-{}", handoff.id)),
        };
        
        self.repository.log_event(event).await
    }
}
```

### 2. Add Workflow and Handoff Parameters
In `mcp-protocol/src/params.rs`:

```rust
use core::models::{WorkflowStep, RetryPolicy};

// Workflow Parameters
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWorkflowParams {
    pub name: String,
    pub description: String,
    pub created_by: String,
    pub steps: Vec<WorkflowStep>,
    pub parallel_execution: Option<bool>,
    pub timeout_minutes: Option<i32>,
    pub retry_policy: Option<RetryPolicy>,
}

impl CreateWorkflowParams {
    pub fn validate(&self) -> Result<()> {
        if self.steps.is_empty() {
            return Err(TaskError::Validation("Workflow must have at least one step".to_string()));
        }
        
        for (idx, step) in self.steps.iter().enumerate() {
            if step.required_capabilities.is_empty() {
                return Err(TaskError::Validation(
                    format!("Step {} must have at least one required capability", idx)
                ));
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssignWorkflowParams {
    pub workflow_id: i32,
    pub task_code: String,
    pub task_name: String,
    pub task_description: String,
    pub initial_agent: Option<String>,
    pub parent_task_id: Option<i32>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdvanceWorkflowParams {
    pub task_code: String,
    pub completed_by: String,
    pub next_agent: Option<String>,
    pub completion_notes: Option<String>,
}

// Handoff Parameters
#[derive(Debug, Clone, Deserialize)]
pub struct CreateHandoffParams {
    pub task_code: String,
    pub from_agent_name: String,
    pub to_agent_name: Option<String>,
    pub to_capability: Option<String>,
    pub context: String,
    pub confidence_score: Option<f64>,
    pub recommended_next_steps: Option<String>,
    pub estimated_effort: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcceptHandoffParams {
    pub handoff_id: i32,
    pub accepting_agent_name: String,
    pub import_knowledge: Option<bool>,
    pub acceptance_notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RejectHandoffParams {
    pub handoff_id: i32,
    pub rejecting_agent_name: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListHandoffsParams {
    pub task_code: Option<String>,
    pub from_agent: Option<String>,
    pub to_agent: Option<String>,
    pub status: Option<String>, // pending, accepted, rejected
    pub since: Option<String>,
    pub limit: Option<i32>,
}

// Task Decomposition Parameters
#[derive(Debug, Clone, Deserialize)]
pub struct DecomposeTaskParams {
    pub parent_task_code: String,
    pub decomposing_agent_name: String,
    pub subtasks: Vec<SubtaskDefinition>,
    pub parent_state_after: Option<String>, // waiting, blocked, in_progress
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubtaskDefinition {
    pub name: String,
    pub description: String,
    pub assigned_agent: Option<String>,
    pub required_capabilities: Option<Vec<String>>,
    pub estimated_effort_minutes: Option<i32>,
    pub priority: Option<i32>,
    pub dependencies: Option<Vec<String>>,
}

// Response Types
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowAdvanceResult {
    pub task_code: String,
    pub current_step: i32,
    pub next_step: Option<i32>,
    pub workflow_complete: bool,
    pub next_agent: Option<String>,
    pub handoff_created: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStatus {
    pub workflow_id: i32,
    pub workflow_name: String,
    pub current_step: i32,
    pub total_steps: i32,
    pub completion_percentage: f64,
    pub current_agent: String,
    pub started_at: DateTime<Utc>,
    pub estimated_completion: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandoffSummary {
    pub pending_handoffs: i32,
    pub accepted_today: i32,
    pub rejected_today: i32,
    pub average_acceptance_time_minutes: f64,
    pub top_capabilities_requested: Vec<(String, i32)>,
}
```

### 3. Create Workflow Orchestration Service
In `mcp-protocol/src/services/workflow_orchestrator.rs`:

```rust
use core::{models::*, repository::TaskRepository};
use std::sync::Arc;

pub struct WorkflowOrchestrator<R: TaskRepository> {
    repository: Arc<R>,
}

impl<R: TaskRepository> WorkflowOrchestrator<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
    
    /// Execute a workflow step
    pub async fn execute_workflow_step(
        &self,
        task: &Task,
        step_index: usize,
    ) -> Result<WorkflowStepResult> {
        let workflow_id = task.workflow_definition_id
            .ok_or_else(|| TaskError::Validation("Task has no workflow".to_string()))?;
        
        let workflow = self.repository
            .get_workflow_definition(workflow_id)
            .await?
            .ok_or_else(|| TaskError::NotFound("Workflow not found".to_string()))?;
        
        if step_index >= workflow.steps.len() {
            return Err(TaskError::Validation("Invalid step index".to_string()));
        }
        
        let step = &workflow.steps[step_index];
        
        // Check prerequisites
        if !self.check_prerequisites(task, step).await? {
            return Ok(WorkflowStepResult {
                step_index,
                status: StepStatus::Blocked,
                assigned_agent: None,
                error: Some("Prerequisites not met".to_string()),
            });
        }
        
        // Find suitable agent
        let agent_rec = self.find_agent_for_step(step).await?;
        
        if let Some(agent) = agent_rec {
            // Create work item for agent
            self.create_step_work_item(task, step, &agent.agent_name).await?;
            
            Ok(WorkflowStepResult {
                step_index,
                status: StepStatus::Assigned,
                assigned_agent: Some(agent.agent_name),
                error: None,
            })
        } else {
            Ok(WorkflowStepResult {
                step_index,
                status: StepStatus::NoAgentAvailable,
                assigned_agent: None,
                error: Some("No suitable agent found".to_string()),
            })
        }
    }
    
    /// Monitor workflow progress
    pub async fn get_workflow_progress(&self, task_code: &str) -> Result<WorkflowProgress> {
        let task = self.repository
            .get_task_by_code(task_code)
            .await?
            .ok_or_else(|| TaskError::NotFound("Task not found".to_string()))?;
        
        let workflow_id = task.workflow_definition_id
            .ok_or_else(|| TaskError::Validation("Task has no workflow".to_string()))?;
        
        let workflow = self.repository
            .get_workflow_definition(workflow_id)
            .await?
            .ok_or_else(|| TaskError::NotFound("Workflow not found".to_string()))?;
        
        let current_step = task.workflow_cursor.unwrap_or(0) as usize;
        let total_steps = workflow.steps.len();
        
        // Calculate time estimates
        let completed_time: i32 = workflow.steps[..current_step].iter()
            .map(|s| s.estimated_minutes.unwrap_or(60))
            .sum();
        
        let remaining_time: i32 = workflow.steps[current_step..].iter()
            .map(|s| s.estimated_minutes.unwrap_or(60))
            .sum();
        
        Ok(WorkflowProgress {
            task_code: task_code.to_string(),
            workflow_name: workflow.name,
            current_step,
            total_steps,
            completion_percentage: (current_step as f64 / total_steps as f64) * 100.0,
            completed_steps: self.get_completed_steps(&task, &workflow).await?,
            current_step_info: if current_step < total_steps {
                Some(workflow.steps[current_step].clone())
            } else {
                None
            },
            estimated_completion_minutes: remaining_time,
            blockers: self.get_workflow_blockers(&task).await?,
        })
    }
    
    async fn check_prerequisites(&self, task: &Task, step: &WorkflowStep) -> Result<bool> {
        // Check if previous steps are complete
        // Check if required resources are available
        // Check if dependencies are satisfied
        Ok(true) // Simplified for now
    }
    
    async fn find_agent_for_step(&self, step: &WorkflowStep) -> Result<Option<AgentRecommendation>> {
        let discovery = AgentDiscoveryService::new(Arc::clone(&self.repository));
        
        // Create mock task for agent discovery
        let mock_task = Task {
            id: 0,
            code: "workflow-step".to_string(),
            name: step.name.clone(),
            description: step.description.clone(),
            required_capabilities: Some(serde_json::to_string(&step.required_capabilities).unwrap()),
            ..Default::default()
        };
        
        discovery.find_best_agent_for_task(&mock_task).await
    }
    
    async fn create_step_work_item(
        &self,
        task: &Task,
        step: &WorkflowStep,
        agent_name: &str,
    ) -> Result<()> {
        // This would create a work notification or assignment
        // For now, just log it
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "workflow_step_assigned".to_string(),
            actor_type: ActorType::System,
            actor_id: "workflow-orchestrator".to_string(),
            task_code: Some(task.code.clone()),
            payload: serde_json::json!({
                "step_name": step.name,
                "assigned_to": agent_name,
                "estimated_minutes": step.estimated_minutes,
            }),
            correlation_id: task.workflow_definition_id
                .map(|id| format!("workflow-{}", id)),
        };
        
        self.repository.log_event(event).await
    }
    
    async fn get_completed_steps(
        &self,
        task: &Task,
        workflow: &WorkflowDefinition,
    ) -> Result<Vec<CompletedStep>> {
        let current = task.workflow_cursor.unwrap_or(0) as usize;
        let mut completed = Vec::new();
        
        for i in 0..current {
            completed.push(CompletedStep {
                step_index: i,
                step_name: workflow.steps[i].name.clone(),
                completed_at: None, // Would need to track this
                completed_by: None, // Would need to track this
                duration_minutes: None,
            });
        }
        
        Ok(completed)
    }
    
    async fn get_workflow_blockers(&self, task: &Task) -> Result<Vec<String>> {
        // Check for unresolved help requests
        let help_requests = self.repository
            .list_help_requests(HelpRequestFilter {
                task_code: Some(task.code.clone()),
                status: Some(HelpRequestStatus::Open),
                ..Default::default()
            })
            .await?;
        
        help_requests.into_iter()
            .filter(|hr| hr.help_type == HelpType::Blocker)
            .map(|hr| hr.description)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowStepResult {
    pub step_index: usize,
    pub status: StepStatus,
    pub assigned_agent: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum StepStatus {
    Ready,
    Assigned,
    InProgress,
    Blocked,
    Complete,
    Failed,
    NoAgentAvailable,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowProgress {
    pub task_code: String,
    pub workflow_name: String,
    pub current_step: usize,
    pub total_steps: usize,
    pub completion_percentage: f64,
    pub completed_steps: Vec<CompletedStep>,
    pub current_step_info: Option<WorkflowStep>,
    pub estimated_completion_minutes: i32,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompletedStep {
    pub step_index: usize,
    pub step_name: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub completed_by: Option<String>,
    pub duration_minutes: Option<i32>,
}
```

## Files to Create/Modify
- `mcp-protocol/src/handler.rs` - Add workflow and handoff handlers
- `mcp-protocol/src/params.rs` - Add parameter types
- `mcp-protocol/src/services/workflow_orchestrator.rs` - Workflow orchestration service
- `mcp-protocol/src/router.rs` - Add method routing

## Testing Requirements
1. Test workflow creation and validation
2. Test workflow advancement
3. Test handoff creation and acceptance
4. Test task decomposition
5. Test knowledge snapshot creation
6. Test parallel workflow execution
7. Test workflow monitoring

## Notes
- Workflows enable multi-step task execution
- Handoffs preserve context and knowledge
- Task decomposition creates subtask hierarchies
- Knowledge snapshots transfer context
- Workflow orchestration handles step sequencing