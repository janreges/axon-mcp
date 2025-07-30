# CORE04: Define Workflow Types

## Objective
Create workflow-related types that enable task orchestration and step-by-step execution tracking in the MCP v2 system.

## Implementation Details

### 1. Create WorkflowDefinition Struct
Create in `core/src/models/workflows.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub transitions: serde_json::Value,  // Step transition rules as JSON
    pub created_by: String,              // Agent or human who created it
    pub is_template: bool,               // Can be reused for similar tasks
    pub created_at: DateTime<Utc>,
}

impl WorkflowDefinition {
    /// Get the first step in the workflow
    pub fn first_step(&self) -> Option<&WorkflowStep> {
        self.steps.first()
    }
    
    /// Find a step by ID
    pub fn find_step(&self, step_id: &str) -> Option<&WorkflowStep> {
        self.steps.iter().find(|s| s.id == step_id)
    }
    
    /// Get the next step after the given step ID
    pub fn next_step(&self, current_step_id: &str) -> Option<&WorkflowStep> {
        let current_index = self.steps.iter().position(|s| s.id == current_step_id)?;
        self.steps.get(current_index + 1)
    }
    
    /// Validate workflow has no orphaned steps
    pub fn validate(&self) -> Result<(), TaskError> {
        if self.steps.is_empty() {
            return Err(TaskError::Validation("Workflow must have at least one step".to_string()));
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
        
        Ok(())
    }
}
```

### 2. Create WorkflowStep Struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,                      // Unique within workflow (e.g., "design", "implement")
    pub name: String,
    pub required_capability: String,     // What kind of agent can do this
    pub estimated_duration: Option<i32>, // Minutes
    pub exit_conditions: Vec<String>,    // When this step is complete
    pub validation_rules: Vec<String>,   // Quality gates
    pub handoff_template: Option<String>, // Template for handoff message
}

impl WorkflowStep {
    /// Check if this step can be performed by an agent with given capabilities
    pub fn can_be_performed_by(&self, capabilities: &[String]) -> bool {
        capabilities.iter().any(|c| c == &self.required_capability)
    }
}
```

### 3. Create Handoff Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffPackage {
    pub id: i32,
    pub task_code: String,
    pub from_agent: String,              // kebab-case agent name
    pub to_capability: String,           // Required capability for next agent
    pub summary: String,
    pub confidence_score: f64,           // 0.0 to 1.0
    pub artifacts: serde_json::Value,    // Links, files, etc.
    pub known_limitations: Vec<String>,
    pub next_steps_suggestion: String,
    pub blockers_resolved: Vec<String>,
    pub estimated_effort: Option<i32>,   // Minutes
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<String>,
}

impl HandoffPackage {
    /// Check if handoff meets minimum confidence threshold
    pub fn meets_confidence_threshold(&self, threshold: f64) -> bool {
        self.confidence_score >= threshold
    }
    
    /// Check if handoff has been accepted
    pub fn is_accepted(&self) -> bool {
        self.accepted_at.is_some() && self.accepted_by.is_some()
    }
}
```

### 4. Create Workflow Execution Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub task_code: String,
    pub workflow_id: i32,
    pub current_step_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_steps: Vec<CompletedStep>,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStep {
    pub step_id: String,
    pub completed_by: String,           // Agent who completed it
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_minutes: i32,
    pub output_summary: String,
    pub confidence_score: f64,
}
```

### 5. Create Workflow Advancement Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAdvanceRequest {
    pub task_code: String,
    pub agent_name: String,
    pub output_summary: String,
    pub confidence_score: f64,
    pub artifacts: serde_json::Value,
    pub next_step_guidance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowAdvanceResult {
    Advanced {
        next_step: WorkflowStep,
        handoff_package: HandoffPackage,
    },
    Completed {
        final_output: String,
        total_duration_minutes: i32,
    },
    ValidationFailed {
        reason: String,
        required_fixes: Vec<String>,
    },
}
```

### 6. Create Task Decomposition Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskPlan {
    pub code_prefix: String,            // e.g., "IMPL" for "IMPL-001", "IMPL-002"
    pub name: String,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub estimated_effort: Option<i32>,
    pub depends_on: Vec<String>,        // Codes of other subtasks
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHierarchy {
    pub parent: Task,
    pub subtasks: Vec<Task>,
    pub dependency_graph: serde_json::Value,  // DAG of dependencies
}
```

### 7. Add Helper Methods and Constants
```rust
pub mod workflow_constants {
    pub const MIN_CONFIDENCE_SCORE: f64 = 0.7;
    pub const MAX_STEP_NAME_LENGTH: usize = 100;
    pub const MAX_HANDOFF_SUMMARY_LENGTH: usize = 2000;
    
    /// Default workflow step IDs
    pub const STEP_PLANNING: &str = "planning";
    pub const STEP_IMPLEMENTATION: &str = "implementation";
    pub const STEP_TESTING: &str = "testing";
    pub const STEP_REVIEW: &str = "review";
    pub const STEP_DEPLOYMENT: &str = "deployment";
}

impl WorkflowStep {
    /// Create a simple step with basic configuration
    pub fn simple(id: &str, name: &str, capability: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            required_capability: capability.to_string(),
            estimated_duration: None,
            exit_conditions: vec![],
            validation_rules: vec![],
            handoff_template: None,
        }
    }
}
```

## Files to Create/Modify
- `core/src/models/workflows.rs` - New file with all workflow types
- `core/src/models.rs` - Add `pub mod workflows;`
- `core/src/lib.rs` - Export workflow types

## Testing Requirements
1. Unit tests for workflow validation
2. Tests for step navigation (first_step, next_step)
3. Tests for handoff confidence threshold checking
4. Serialization/deserialization tests
5. Tests for workflow execution tracking

## Integration Notes
- WorkflowDefinition will be stored in `workflows` table
- HandoffPackage will be stored in `handoffs` table
- Workflow transitions will use JSON for flexibility
- Step IDs should be human-readable (e.g., "design", "implement", not UUIDs)