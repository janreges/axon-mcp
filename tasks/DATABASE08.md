# DATABASE08: Implement Workflow Repository

## Objective
Implement all workflow management methods in the SQLite repository, including workflow creation, execution tracking, step advancement, and handoff management.

## Implementation Details

### 1. Add Workflow Methods to SqliteTaskRepository
In `database/src/sqlite.rs`, add implementations for workflow-related methods:

```rust
impl TaskRepository for SqliteTaskRepository {
    // ... existing implementations ...
    
    async fn create_workflow(&self, workflow: NewWorkflowDefinition) -> Result<WorkflowDefinition> {
        // Validate input
        workflow.validate()?;
        
        // Convert steps and transitions to JSON
        let steps_json = serde_json::to_string(&workflow.steps)
            .map_err(|e| TaskError::Serialization(format!("Failed to serialize steps: {}", e)))?;
        let transitions_json = serde_json::to_string(&workflow.transitions)
            .map_err(|e| TaskError::Serialization(format!("Failed to serialize transitions: {}", e)))?;
        
        // Insert workflow
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO workflows 
            (name, description, steps, transitions, created_by, is_template)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(&workflow.name)
        .bind(&workflow.description)
        .bind(&steps_json)
        .bind(&transitions_json)
        .bind(&workflow.created_by)
        .bind(workflow.is_template)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Fetch and return created workflow
        self.get_workflow(id)
            .await?
            .ok_or_else(|| TaskError::Database("Failed to fetch created workflow".to_string()))
    }
    
    async fn get_workflow(&self, workflow_id: i32) -> Result<Option<WorkflowDefinition>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, steps, transitions, 
                   created_by, is_template, created_at
            FROM workflows
            WHERE id = ?
            "#
        )
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        row.map(|r| self.row_to_workflow_definition(r)).transpose()
    }
    
    async fn list_workflow_templates(&self) -> Result<Vec<WorkflowDefinition>> {
        let workflows = sqlx::query(
            r#"
            SELECT id, name, description, steps, transitions, 
                   created_by, is_template, created_at
            FROM workflows
            WHERE is_template = 1
            ORDER BY name
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        workflows.into_iter()
            .map(|row| self.row_to_workflow_definition(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn assign_workflow_to_task(&self, task_code: &str, workflow_id: i32) -> Result<()> {
        // Verify workflow exists
        let workflow = self.get_workflow(workflow_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Workflow {} not found", workflow_id)))?;
        
        // Get first step ID
        let first_step_id = workflow.first_step()
            .map(|s| s.id.clone())
            .ok_or_else(|| TaskError::Validation("Workflow has no steps".to_string()))?;
        
        // Update task with workflow
        let affected = sqlx::query(
            r#"
            UPDATE tasks 
            SET workflow_definition_id = ?, 
                workflow_cursor = ?,
                state = 'InProgress'
            WHERE code = ?
            "#
        )
        .bind(workflow_id)
        .bind(&first_step_id)
        .bind(task_code)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .rows_affected();
        
        if affected == 0 {
            return Err(TaskError::NotFound(format!("Task {} not found", task_code)));
        }
        
        Ok(())
    }
    
    async fn advance_workflow(&self, request: WorkflowAdvanceRequest) -> Result<WorkflowAdvanceResult> {
        // Start transaction
        let mut tx = self.pool.begin().await.map_err(sqlx_error_to_task_error)?;
        
        // Get task with workflow info
        let task_row = sqlx::query(
            r#"
            SELECT workflow_definition_id, workflow_cursor
            FROM tasks
            WHERE code = ?
            "#
        )
        .bind(&request.task_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(sqlx_error_to_task_error)?
        .ok_or_else(|| TaskError::NotFound(format!("Task {} not found", request.task_code)))?;
        
        let workflow_id: Option<i32> = task_row.get("workflow_definition_id");
        let current_step_id: Option<String> = task_row.get("workflow_cursor");
        
        let workflow_id = workflow_id
            .ok_or_else(|| TaskError::Validation("Task has no workflow assigned".to_string()))?;
        let current_step_id = current_step_id
            .ok_or_else(|| TaskError::Validation("Task has no current workflow step".to_string()))?;
        
        // Get workflow definition
        let workflow = self.get_workflow(workflow_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Workflow {} not found", workflow_id)))?;
        
        // Validate current step
        let current_step = workflow.find_step(&current_step_id)
            .ok_or_else(|| TaskError::Validation(format!("Step {} not found in workflow", current_step_id)))?;
        
        // Record step completion
        let completion = WorkflowStepCompletion {
            task_code: request.task_code.clone(),
            step_id: current_step_id.clone(),
            completed_by: request.agent_name.clone(),
            output_summary: request.output_summary.clone(),
            confidence_score: request.confidence_score,
            duration_minutes: 0, // Would calculate from work sessions
            artifacts: request.artifacts.clone(),
        };
        
        self.complete_workflow_step(completion).await?;
        
        // Check for next step
        if let Some(next_step) = workflow.next_step(&current_step_id) {
            // Update task cursor
            sqlx::query(
                "UPDATE tasks SET workflow_cursor = ? WHERE code = ?"
            )
            .bind(&next_step.id)
            .bind(&request.task_code)
            .execute(&mut *tx)
            .await
            .map_err(sqlx_error_to_task_error)?;
            
            // Create handoff package
            let handoff = NewHandoffPackage {
                task_code: request.task_code.clone(),
                from_agent: request.agent_name.clone(),
                to_capability: next_step.required_capability.clone(),
                summary: request.output_summary.clone(),
                confidence_score: request.confidence_score,
                artifacts: request.artifacts,
                known_limitations: vec![],
                next_steps_suggestion: request.next_step_guidance.unwrap_or_default(),
                blockers_resolved: vec![],
                estimated_effort: next_step.estimated_duration,
            };
            
            let handoff_package = self.create_handoff(handoff).await?;
            
            // Update task state to pending handoff
            sqlx::query(
                "UPDATE tasks SET state = 'PendingHandoff' WHERE code = ?"
            )
            .bind(&request.task_code)
            .execute(&mut *tx)
            .await
            .map_err(sqlx_error_to_task_error)?;
            
            tx.commit().await.map_err(sqlx_error_to_task_error)?;
            
            Ok(WorkflowAdvanceResult::Advanced {
                next_step: next_step.clone(),
                handoff_package,
            })
        } else {
            // Workflow complete
            sqlx::query(
                "UPDATE tasks SET state = 'Done', done_at = CURRENT_TIMESTAMP WHERE code = ?"
            )
            .bind(&request.task_code)
            .execute(&mut *tx)
            .await
            .map_err(sqlx_error_to_task_error)?;
            
            // Calculate total duration
            let total_duration = sqlx::query_scalar::<_, i32>(
                r#"
                SELECT COALESCE(SUM(duration_minutes), 0)
                FROM completed_workflow_steps
                WHERE task_code = ?
                "#
            )
            .bind(&request.task_code)
            .fetch_one(&mut *tx)
            .await
            .map_err(sqlx_error_to_task_error)?;
            
            tx.commit().await.map_err(sqlx_error_to_task_error)?;
            
            Ok(WorkflowAdvanceResult::Completed {
                final_output: request.output_summary,
                total_duration_minutes: total_duration,
            })
        }
    }
    
    async fn get_workflow_execution(&self, task_code: &str) -> Result<Option<WorkflowExecution>> {
        let task_row = sqlx::query(
            r#"
            SELECT workflow_definition_id, workflow_cursor, inserted_at
            FROM tasks
            WHERE code = ?
            "#
        )
        .bind(task_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if let Some(row) = task_row {
            let workflow_id: Option<i32> = row.get("workflow_definition_id");
            let current_step_id: Option<String> = row.get("workflow_cursor");
            let started_at: DateTime<Utc> = row.get("inserted_at");
            
            if let (Some(wf_id), Some(step_id)) = (workflow_id, current_step_id) {
                // Get completed steps
                let completed_steps = self.get_workflow_history(task_code).await?;
                let is_complete = completed_steps.iter()
                    .any(|s| s.step_id == step_id && s.completed_at.is_some());
                
                return Ok(Some(WorkflowExecution {
                    task_code: task_code.to_string(),
                    workflow_id: wf_id,
                    current_step_id: step_id,
                    started_at,
                    completed_steps,
                    is_complete,
                }));
            }
        }
        
        Ok(None)
    }
    
    async fn complete_workflow_step(&self, completion: WorkflowStepCompletion) -> Result<()> {
        completion.validate()?;
        
        let artifacts_json = serde_json::to_string(&completion.artifacts)
            .map_err(|e| TaskError::Serialization(format!("Failed to serialize artifacts: {}", e)))?;
        
        // Insert into completed steps table (would need to create this table)
        sqlx::query(
            r#"
            INSERT INTO completed_workflow_steps
            (task_code, step_id, completed_by, completed_at, 
             output_summary, confidence_score, duration_minutes, artifacts)
            VALUES (?, ?, ?, CURRENT_TIMESTAMP, ?, ?, ?, ?)
            "#
        )
        .bind(&completion.task_code)
        .bind(&completion.step_id)
        .bind(&completion.completed_by)
        .bind(&completion.output_summary)
        .bind(completion.confidence_score)
        .bind(completion.duration_minutes)
        .bind(&artifacts_json)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        Ok(())
    }
    
    async fn get_workflow_history(&self, task_code: &str) -> Result<Vec<CompletedStep>> {
        let steps = sqlx::query(
            r#"
            SELECT step_id, completed_by, started_at, completed_at,
                   duration_minutes, output_summary, confidence_score
            FROM completed_workflow_steps
            WHERE task_code = ?
            ORDER BY completed_at
            "#
        )
        .bind(task_code)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        steps.into_iter()
            .map(|row| {
                Ok(CompletedStep {
                    step_id: row.get("step_id"),
                    completed_by: row.get("completed_by"),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    duration_minutes: row.get("duration_minutes"),
                    output_summary: row.get("output_summary"),
                    confidence_score: row.get("confidence_score"),
                })
            })
            .collect::<Result<Vec<_>>>()
    }
    
    async fn clone_workflow_template(&self, workflow_id: i32, new_name: &str) -> Result<WorkflowDefinition> {
        let original = self.get_workflow(workflow_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Workflow {} not found", workflow_id)))?;
        
        if !original.is_template {
            return Err(TaskError::Validation("Can only clone template workflows".to_string()));
        }
        
        let new_workflow = NewWorkflowDefinition {
            name: new_name.to_string(),
            description: format!("Cloned from: {}", original.description),
            steps: original.steps,
            transitions: original.transitions,
            created_by: "system".to_string(), // Or current user
            is_template: true,
        };
        
        self.create_workflow(new_workflow).await
    }
}
```

### 2. Add Handoff Methods Implementation
```rust
impl TaskRepository for SqliteTaskRepository {
    async fn create_handoff(&self, handoff: NewHandoffPackage) -> Result<HandoffPackage> {
        handoff.validate()?;
        
        // Validate task exists
        let task_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE code = ?)"
        )
        .bind(&handoff.task_code)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if !task_exists {
            return Err(TaskError::NotFound(format!("Task {} not found", handoff.task_code)));
        }
        
        // Validate from agent exists
        let agent_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE name = ?)"
        )
        .bind(&handoff.from_agent)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if !agent_exists {
            return Err(TaskError::NotFound(format!("Agent {} not found", handoff.from_agent)));
        }
        
        // Convert arrays to JSON
        let artifacts_json = serde_json::to_string(&handoff.artifacts)
            .map_err(|e| TaskError::Serialization(format!("Failed to serialize artifacts: {}", e)))?;
        let limitations_json = serde_json::to_string(&handoff.known_limitations)
            .map_err(|e| TaskError::Serialization(format!("Failed to serialize limitations: {}", e)))?;
        let blockers_json = serde_json::to_string(&handoff.blockers_resolved)
            .map_err(|e| TaskError::Serialization(format!("Failed to serialize blockers: {}", e)))?;
        
        // Insert handoff
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO handoffs
            (task_code, from_agent_name, to_capability, summary, confidence_score,
             artifacts, known_limitations, next_steps_suggestion, blockers_resolved,
             estimated_effort)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(&handoff.task_code)
        .bind(&handoff.from_agent)
        .bind(&handoff.to_capability)
        .bind(&handoff.summary)
        .bind(handoff.confidence_score)
        .bind(&artifacts_json)
        .bind(&limitations_json)
        .bind(&handoff.next_steps_suggestion)
        .bind(&blockers_json)
        .bind(handoff.estimated_effort)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Fetch and return
        self.get_handoff(id)
            .await?
            .ok_or_else(|| TaskError::Database("Failed to fetch created handoff".to_string()))
    }
    
    async fn get_task_handoffs(&self, task_code: &str) -> Result<Vec<HandoffPackage>> {
        let handoffs = sqlx::query(
            r#"
            SELECT id, task_code, from_agent_name, to_capability, summary,
                   confidence_score, artifacts, known_limitations, next_steps_suggestion,
                   blockers_resolved, estimated_effort, created_at, accepted_at, accepted_by
            FROM handoffs
            WHERE task_code = ?
            ORDER BY created_at DESC
            "#
        )
        .bind(task_code)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        handoffs.into_iter()
            .map(|row| self.row_to_handoff_package(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_pending_handoffs(&self, capability: &str) -> Result<Vec<HandoffPackage>> {
        let handoffs = sqlx::query(
            r#"
            SELECT h.id, h.task_code, h.from_agent_name, h.to_capability, h.summary,
                   h.confidence_score, h.artifacts, h.known_limitations, h.next_steps_suggestion,
                   h.blockers_resolved, h.estimated_effort, h.created_at, h.accepted_at, h.accepted_by
            FROM handoffs h
            INNER JOIN tasks t ON t.code = h.task_code
            WHERE h.to_capability = ?
              AND h.accepted_at IS NULL
              AND t.state = 'PendingHandoff'
            ORDER BY h.confidence_score DESC, h.created_at ASC
            "#
        )
        .bind(capability)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        handoffs.into_iter()
            .map(|row| self.row_to_handoff_package(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn accept_handoff(&self, handoff_id: i32, agent_name: &str) -> Result<()> {
        // Start transaction
        let mut tx = self.pool.begin().await.map_err(sqlx_error_to_task_error)?;
        
        // Get handoff details
        let handoff_row = sqlx::query(
            "SELECT task_code, accepted_at FROM handoffs WHERE id = ?"
        )
        .bind(handoff_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(sqlx_error_to_task_error)?
        .ok_or_else(|| TaskError::NotFound(format!("Handoff {} not found", handoff_id)))?;
        
        let task_code: String = handoff_row.get("task_code");
        let accepted_at: Option<DateTime<Utc>> = handoff_row.get("accepted_at");
        
        if accepted_at.is_some() {
            return Err(TaskError::Validation("Handoff already accepted".to_string()));
        }
        
        // Verify agent exists and has capability
        let agent = self.get_agent(agent_name)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", agent_name)))?;
        
        // Update handoff
        sqlx::query(
            r#"
            UPDATE handoffs 
            SET accepted_at = CURRENT_TIMESTAMP, accepted_by = ?
            WHERE id = ?
            "#
        )
        .bind(agent_name)
        .bind(handoff_id)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Update task owner and state
        sqlx::query(
            r#"
            UPDATE tasks 
            SET owner_agent_name = ?, state = 'InProgress'
            WHERE code = ?
            "#
        )
        .bind(agent_name)
        .bind(&task_code)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        tx.commit().await.map_err(sqlx_error_to_task_error)?;
        
        Ok(())
    }
    
    async fn get_handoff(&self, handoff_id: i32) -> Result<Option<HandoffPackage>> {
        let row = sqlx::query(
            r#"
            SELECT id, task_code, from_agent_name, to_capability, summary,
                   confidence_score, artifacts, known_limitations, next_steps_suggestion,
                   blockers_resolved, estimated_effort, created_at, accepted_at, accepted_by
            FROM handoffs
            WHERE id = ?
            "#
        )
        .bind(handoff_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        row.map(|r| self.row_to_handoff_package(r)).transpose()
    }
}
```

### 3. Add Task Decomposition Methods
```rust
impl TaskRepository for SqliteTaskRepository {
    async fn decompose_task(&self, parent_code: &str, subtasks: Vec<SubtaskPlan>) -> Result<Vec<Task>> {
        // Validate parent exists
        let parent_id = sqlx::query_scalar::<_, i32>(
            "SELECT id FROM tasks WHERE code = ?"
        )
        .bind(parent_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .ok_or_else(|| TaskError::NotFound(format!("Parent task {} not found", parent_code)))?;
        
        let mut created_tasks = Vec::new();
        
        // Start transaction
        let mut tx = self.pool.begin().await.map_err(sqlx_error_to_task_error)?;
        
        for (index, plan) in subtasks.iter().enumerate() {
            let subtask_code = format!("{}-{:03}", plan.code_prefix, index + 1);
            
            // Convert required capabilities to JSON
            let capabilities_json = serde_json::to_string(&plan.required_capabilities)
                .map_err(|e| TaskError::Serialization(format!("Failed to serialize capabilities: {}", e)))?;
            
            // Create subtask
            let new_task = NewTask {
                code: subtask_code.clone(),
                name: plan.name.clone(),
                description: plan.description.clone(),
                owner_agent_name: "unassigned".to_string(), // Will be assigned based on capabilities
            };
            
            // Insert subtask
            sqlx::query(
                r#"
                INSERT INTO tasks 
                (code, name, description, owner_agent_name, state, parent_task_id, required_capabilities)
                VALUES (?, ?, ?, ?, 'Created', ?, ?)
                "#
            )
            .bind(&new_task.code)
            .bind(&new_task.name)
            .bind(&new_task.description)
            .bind(&new_task.owner_agent_name)
            .bind(parent_id)
            .bind(&capabilities_json)
            .execute(&mut *tx)
            .await
            .map_err(sqlx_error_to_task_error)?;
            
            // TODO: Handle dependencies between subtasks
            
            // Fetch created task
            let task = self.get_task_by_code(&subtask_code)
                .await?
                .ok_or_else(|| TaskError::Database("Failed to fetch created subtask".to_string()))?;
            
            created_tasks.push(task);
        }
        
        // Update parent task state
        sqlx::query(
            "UPDATE tasks SET state = 'PendingDecomposition' WHERE code = ?"
        )
        .bind(parent_code)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        tx.commit().await.map_err(sqlx_error_to_task_error)?;
        
        Ok(created_tasks)
    }
    
    async fn get_task_hierarchy(&self, parent_code: &str) -> Result<TaskHierarchy> {
        // Get parent task
        let parent = self.get_task_by_code(parent_code)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Task {} not found", parent_code)))?;
        
        // Get subtasks
        let subtasks = self.get_subtasks(parent_code).await?;
        
        // Build dependency graph (simplified - would need proper implementation)
        let dependency_graph = serde_json::json!({
            "nodes": subtasks.iter().map(|t| &t.code).collect::<Vec<_>>(),
            "edges": []  // TODO: Implement dependency tracking
        });
        
        Ok(TaskHierarchy {
            parent,
            subtasks,
            dependency_graph,
        })
    }
    
    async fn get_subtasks(&self, parent_code: &str) -> Result<Vec<Task>> {
        let parent_id = sqlx::query_scalar::<_, i32>(
            "SELECT id FROM tasks WHERE code = ?"
        )
        .bind(parent_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .ok_or_else(|| TaskError::NotFound(format!("Parent task {} not found", parent_code)))?;
        
        let subtasks = sqlx::query("SELECT * FROM tasks WHERE parent_task_id = ?")
            .bind(parent_id)
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        subtasks.into_iter()
            .map(|row| self.row_to_task(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn update_parent_progress(&self, subtask_code: &str) -> Result<()> {
        // Get parent task ID from subtask
        let parent_id = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT parent_task_id FROM tasks WHERE code = ?"
        )
        .bind(subtask_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .flatten();
        
        if let Some(parent_id) = parent_id {
            // Check if all subtasks are done
            let all_done = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT NOT EXISTS(
                    SELECT 1 FROM tasks 
                    WHERE parent_task_id = ? 
                      AND state != 'Done'
                )
                "#
            )
            .bind(parent_id)
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
            
            if all_done {
                // Mark parent as done
                sqlx::query(
                    "UPDATE tasks SET state = 'Done', done_at = CURRENT_TIMESTAMP WHERE id = ?"
                )
                .bind(parent_id)
                .execute(&self.pool)
                .await
                .map_err(sqlx_error_to_task_error)?;
            }
        }
        
        Ok(())
    }
}
```

### 4. Add Helper Methods
```rust
impl SqliteTaskRepository {
    fn row_to_workflow_definition(&self, row: SqliteRow) -> Result<WorkflowDefinition> {
        let steps_json: String = row.get("steps");
        let steps: Vec<WorkflowStep> = serde_json::from_str(&steps_json)
            .map_err(|e| TaskError::Database(format!("Invalid steps JSON: {}", e)))?;
        
        let transitions_json: String = row.get("transitions");
        let transitions = serde_json::from_str(&transitions_json)
            .unwrap_or_else(|_| serde_json::json!({}));
        
        Ok(WorkflowDefinition {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            steps,
            transitions,
            created_by: row.get("created_by"),
            is_template: row.get("is_template"),
            created_at: row.get("created_at"),
        })
    }
    
    fn row_to_handoff_package(&self, row: SqliteRow) -> Result<HandoffPackage> {
        let artifacts_json: String = row.get("artifacts");
        let artifacts = serde_json::from_str(&artifacts_json)
            .unwrap_or_else(|_| serde_json::json!({}));
        
        let limitations_json: String = row.get("known_limitations");
        let known_limitations: Vec<String> = serde_json::from_str(&limitations_json)
            .unwrap_or_default();
        
        let blockers_json: String = row.get("blockers_resolved");
        let blockers_resolved: Vec<String> = serde_json::from_str(&blockers_json)
            .unwrap_or_default();
        
        Ok(HandoffPackage {
            id: row.get("id"),
            task_code: row.get("task_code"),
            from_agent: row.get("from_agent_name"),
            to_capability: row.get("to_capability"),
            summary: row.get("summary"),
            confidence_score: row.get("confidence_score"),
            artifacts,
            known_limitations,
            next_steps_suggestion: row.get("next_steps_suggestion"),
            blockers_resolved,
            estimated_effort: row.get("estimated_effort"),
            created_at: row.get("created_at"),
            accepted_at: row.get("accepted_at"),
            accepted_by: row.get("accepted_by"),
        })
    }
}
```

### 5. Create Workflow Step Tracking Table
Add to migration:

```sql
CREATE TABLE IF NOT EXISTS completed_workflow_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_code TEXT NOT NULL,
    step_id TEXT NOT NULL,
    completed_by TEXT NOT NULL,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    duration_minutes INTEGER NOT NULL,
    output_summary TEXT NOT NULL,
    confidence_score REAL NOT NULL,
    artifacts TEXT,
    
    FOREIGN KEY (task_code) REFERENCES tasks(code) ON DELETE CASCADE,
    FOREIGN KEY (completed_by) REFERENCES agents(name) ON DELETE CASCADE
);

CREATE INDEX idx_workflow_steps_task ON completed_workflow_steps(task_code);
CREATE INDEX idx_workflow_steps_completed ON completed_workflow_steps(completed_at);
```

## Files to Modify
- `database/src/sqlite.rs` - Add workflow method implementations
- `database/migrations/sqlite/007_workflow_steps.sql` - New migration for step tracking

## Testing Requirements
1. Test workflow creation and validation
2. Test workflow assignment to tasks
3. Test step advancement logic
4. Test handoff creation and acceptance
5. Test task decomposition
6. Test parent progress updates
7. Test workflow completion

## Performance Considerations
1. Workflow steps are stored as JSON - consider normalized tables for complex workflows
2. Handoff queries should use indexes on capability and acceptance status
3. Task hierarchy queries may need optimization for deep nesting
4. Consider caching workflow definitions

## Security Considerations
1. Validate all JSON structures before storage
2. Ensure capability matching is strict
3. Prevent workflow cycles in transitions
4. Validate agent permissions for handoff acceptance