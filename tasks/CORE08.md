# CORE08: Extend TaskRepository Trait - Workflows

## Objective
Extend the TaskRepository trait to include methods for workflow management, enabling task orchestration and step-by-step execution tracking.

## Current State
The TaskRepository trait needs workflow methods to support structured task execution and handoffs between agents.

## Required Changes

### 1. Extend TaskRepository Trait
Add to `core/src/repository.rs`:

```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Workflow Management Methods =====
    
    /// Create a new workflow definition
    async fn create_workflow(&self, workflow: NewWorkflowDefinition) -> Result<WorkflowDefinition>;
    
    /// Get workflow by ID
    async fn get_workflow(&self, workflow_id: i32) -> Result<Option<WorkflowDefinition>>;
    
    /// List all workflow templates
    async fn list_workflow_templates(&self) -> Result<Vec<WorkflowDefinition>>;
    
    /// Assign workflow to task
    async fn assign_workflow_to_task(&self, task_code: &str, workflow_id: i32) -> Result<()>;
    
    /// Advance workflow to next step
    async fn advance_workflow(&self, request: WorkflowAdvanceRequest) -> Result<WorkflowAdvanceResult>;
    
    /// Get current workflow execution state
    async fn get_workflow_execution(&self, task_code: &str) -> Result<Option<WorkflowExecution>>;
    
    /// Record workflow step completion
    async fn complete_workflow_step(&self, completion: WorkflowStepCompletion) -> Result<()>;
    
    /// Get workflow history for a task
    async fn get_workflow_history(&self, task_code: &str) -> Result<Vec<CompletedStep>>;
    
    /// Clone workflow template
    async fn clone_workflow_template(&self, workflow_id: i32, new_name: &str) -> Result<WorkflowDefinition>;
}
```

### 2. Add Workflow Creation Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewWorkflowDefinition {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub transitions: serde_json::Value,
    pub created_by: String,
    pub is_template: bool,
}

impl NewWorkflowDefinition {
    pub fn validate(&self) -> Result<()> {
        // Validate name
        if self.name.is_empty() || self.name.len() > 100 {
            return Err(TaskError::Validation(
                "Workflow name must be between 1 and 100 characters".to_string()
            ));
        }
        
        // Validate steps
        if self.steps.is_empty() {
            return Err(TaskError::Validation(
                "Workflow must have at least one step".to_string()
            ));
        }
        
        // Check for duplicate step IDs
        let mut step_ids = HashSet::new();
        for step in &self.steps {
            if !step_ids.insert(&step.id) {
                return Err(TaskError::Validation(
                    format!("Duplicate step ID: {}", step.id)
                ));
            }
        }
        
        // Validate each step
        for step in &self.steps {
            step.validate()?;
        }
        
        Ok(())
    }
}
```

### 3. Add Workflow Step Completion Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepCompletion {
    pub task_code: String,
    pub step_id: String,
    pub completed_by: String,
    pub output_summary: String,
    pub confidence_score: f64,
    pub duration_minutes: i32,
    pub artifacts: serde_json::Value,
}

impl WorkflowStepCompletion {
    pub fn validate(&self) -> Result<()> {
        if self.confidence_score < 0.0 || self.confidence_score > 1.0 {
            return Err(TaskError::Validation(
                "Confidence score must be between 0.0 and 1.0".to_string()
            ));
        }
        
        if self.duration_minutes < 0 {
            return Err(TaskError::Validation(
                "Duration cannot be negative".to_string()
            ));
        }
        
        Ok(())
    }
}
```

### 4. Add Handoff Methods
```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Handoff Methods =====
    
    /// Create a handoff package
    async fn create_handoff(&self, handoff: NewHandoffPackage) -> Result<HandoffPackage>;
    
    /// Get handoffs for a task
    async fn get_task_handoffs(&self, task_code: &str) -> Result<Vec<HandoffPackage>>;
    
    /// Get pending handoffs for a capability
    async fn get_pending_handoffs(&self, capability: &str) -> Result<Vec<HandoffPackage>>;
    
    /// Accept a handoff
    async fn accept_handoff(&self, handoff_id: i32, agent_name: &str) -> Result<()>;
    
    /// Get handoff by ID
    async fn get_handoff(&self, handoff_id: i32) -> Result<Option<HandoffPackage>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewHandoffPackage {
    pub task_code: String,
    pub from_agent: String,
    pub to_capability: String,
    pub summary: String,
    pub confidence_score: f64,
    pub artifacts: serde_json::Value,
    pub known_limitations: Vec<String>,
    pub next_steps_suggestion: String,
    pub blockers_resolved: Vec<String>,
    pub estimated_effort: Option<i32>,
}

impl NewHandoffPackage {
    pub fn validate(&self) -> Result<()> {
        if self.summary.is_empty() || self.summary.len() > 2000 {
            return Err(TaskError::Validation(
                "Handoff summary must be between 1 and 2000 characters".to_string()
            ));
        }
        
        if self.confidence_score < 0.0 || self.confidence_score > 1.0 {
            return Err(TaskError::Validation(
                "Confidence score must be between 0.0 and 1.0".to_string()
            ));
        }
        
        if self.to_capability.is_empty() {
            return Err(TaskError::Validation(
                "Target capability must be specified".to_string()
            ));
        }
        
        Ok(())
    }
}
```

### 5. Add Task Decomposition Methods
```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Task Decomposition Methods =====
    
    /// Decompose a task into subtasks
    async fn decompose_task(&self, parent_code: &str, subtasks: Vec<SubtaskPlan>) -> Result<Vec<Task>>;
    
    /// Get task hierarchy
    async fn get_task_hierarchy(&self, parent_code: &str) -> Result<TaskHierarchy>;
    
    /// Get subtasks of a parent task
    async fn get_subtasks(&self, parent_code: &str) -> Result<Vec<Task>>;
    
    /// Update parent task when subtask completes
    async fn update_parent_progress(&self, subtask_code: &str) -> Result<()>;
}
```

### 6. Protocol Handler Extension
```rust
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    // ... existing methods ...
    
    // Workflows
    async fn create_workflow(&self, params: CreateWorkflowParams) -> Result<WorkflowDefinition>;
    async fn assign_workflow(&self, params: AssignWorkflowParams) -> Result<()>;
    async fn advance_workflow(&self, params: AdvanceWorkflowParams) -> Result<WorkflowAdvanceResult>;
    async fn get_workflow_status(&self, params: GetWorkflowStatusParams) -> Result<WorkflowExecution>;
    
    // Handoffs
    async fn create_handoff(&self, params: CreateHandoffParams) -> Result<HandoffPackage>;
    async fn accept_handoff(&self, params: AcceptHandoffParams) -> Result<()>;
    async fn get_pending_handoffs(&self, params: GetPendingHandoffsParams) -> Result<Vec<HandoffPackage>>;
    
    // Task Decomposition
    async fn decompose_task(&self, params: DecomposeTaskParams) -> Result<Vec<Task>>;
    async fn get_task_hierarchy(&self, params: GetTaskHierarchyParams) -> Result<TaskHierarchy>;
}

// Parameter types
#[derive(Debug, Deserialize)]
pub struct CreateWorkflowParams {
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub created_by: String,
    pub is_template: bool,
}

#[derive(Debug, Deserialize)]
pub struct AssignWorkflowParams {
    pub task_code: String,
    pub workflow_id: i32,
    pub requesting_agent: String,
}

#[derive(Debug, Deserialize)]
pub struct AdvanceWorkflowParams {
    pub task_code: String,
    pub agent_name: String,
    pub output_summary: String,
    pub confidence_score: f64,
    pub artifacts: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateHandoffParams {
    pub task_code: String,
    pub from_agent: String,
    pub to_capability: String,
    pub summary: String,
    pub confidence_score: f64,
    pub artifacts: serde_json::Value,
    pub known_limitations: Vec<String>,
    pub next_steps_suggestion: String,
}

#[derive(Debug, Deserialize)]
pub struct AcceptHandoffParams {
    pub handoff_id: i32,
    pub agent_name: String,
}

#[derive(Debug, Deserialize)]
pub struct DecomposeTaskParams {
    pub parent_task_code: String,
    pub agent_name: String,
    pub subtasks: Vec<SubtaskPlan>,
}
```

### 7. Add Workflow Helper Methods
```rust
impl WorkflowStep {
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() || self.id.len() > 50 {
            return Err(TaskError::Validation(
                "Step ID must be between 1 and 50 characters".to_string()
            ));
        }
        
        if self.name.is_empty() || self.name.len() > 100 {
            return Err(TaskError::Validation(
                "Step name must be between 1 and 100 characters".to_string()
            ));
        }
        
        if self.required_capability.is_empty() {
            return Err(TaskError::Validation(
                "Required capability must be specified".to_string()
            ));
        }
        
        Ok(())
    }
}

impl WorkflowExecution {
    /// Get percentage complete based on steps
    pub fn percentage_complete(&self) -> f64 {
        if self.completed_steps.is_empty() {
            return 0.0;
        }
        // This is approximate - would need total steps from workflow definition
        100.0 // Placeholder
    }
    
    /// Get total time spent so far
    pub fn total_duration_minutes(&self) -> i32 {
        self.completed_steps.iter()
            .map(|s| s.duration_minutes)
            .sum()
    }
}
```

## Files to Modify
- `core/src/repository.rs` - Add workflow methods to trait
- `core/src/protocol.rs` - Add protocol handler methods
- `core/src/models/workflows.rs` - Add validation methods
- `core/src/models.rs` - Ensure workflow types are imported

## Testing Requirements
1. Mock implementations for all new methods
2. Tests for workflow validation
3. Tests for step completion tracking
4. Tests for handoff creation and acceptance
5. Tests for task decomposition
6. Integration tests in database crate

## Notes
- Workflows can be templates (reusable) or instance-specific
- Handoffs require capability matching, not specific agents
- Task decomposition creates parent-child relationships
- Workflow advancement includes validation and quality gates
- Step transitions can be complex (stored as JSON)