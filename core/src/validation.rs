use crate::{
    error::{Result, TaskError},
    models::{Task, TaskState, NewTask},
};

/// Validation utilities for task management operations
pub struct TaskValidator;

impl TaskValidator {
    /// Validate a task code format
    /// 
    /// Task codes must:
    /// - Be 3-20 characters long
    /// - Start with a letter
    /// - Contain only letters, numbers, and hyphens
    /// - Not end with a hyphen
    /// 
    /// # Arguments
    /// * `code` - The task code to validate
    /// 
    /// # Returns
    /// * `Ok(())` - If the code is valid
    /// * `Err(TaskError::Validation)` - If the code is invalid
    pub fn validate_task_code(code: &str) -> Result<()> {
        if code.is_empty() {
            return Err(TaskError::empty_field("code"));
        }

        if code.len() < 3 {
            return Err(TaskError::Validation(
                "Task code must be at least 3 characters long".to_string()
            ));
        }

        if code.len() > 20 {
            return Err(TaskError::Validation(
                "Task code must be at most 20 characters long".to_string()
            ));
        }

        // Must start with a letter
        if !code.chars().next().unwrap().is_alphabetic() {
            return Err(TaskError::Validation(
                "Task code must start with a letter".to_string()
            ));
        }

        // Must not end with a hyphen
        if code.ends_with('-') {
            return Err(TaskError::Validation(
                "Task code must not end with a hyphen".to_string()
            ));
        }

        // Only allow letters, numbers, and hyphens
        let valid_chars = code.chars().all(|c| c.is_alphanumeric() || c == '-');
        if !valid_chars {
            return Err(TaskError::Validation(
                "Task code can only contain letters, numbers, and hyphens".to_string()
            ));
        }

        // No consecutive hyphens
        if code.contains("--") {
            return Err(TaskError::Validation(
                "Task code cannot contain consecutive hyphens".to_string()
            ));
        }

        Ok(())
    }

    /// Validate an agent name
    /// 
    /// Agent names must:
    /// - Be 1-50 characters long
    /// - Contain only letters, numbers, hyphens, and underscores
    /// - Not start or end with special characters
    /// 
    /// # Arguments
    /// * `name` - The agent name to validate
    /// 
    /// # Returns
    /// * `Ok(())` - If the name is valid
    /// * `Err(TaskError::Validation)` - If the name is invalid
    pub fn validate_agent_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(TaskError::empty_field("agent_name"));
        }

        if name.len() > 50 {
            return Err(TaskError::Validation(
                "Agent name must be at most 50 characters long".to_string()
            ));
        }

        // Must start and end with alphanumeric characters
        let first_char = name.chars().next().unwrap();
        let last_char = name.chars().last().unwrap();
        
        if !first_char.is_alphanumeric() {
            return Err(TaskError::Validation(
                "Agent name must start with a letter or number".to_string()
            ));
        }

        if !last_char.is_alphanumeric() {
            return Err(TaskError::Validation(
                "Agent name must end with a letter or number".to_string()
            ));
        }

        // Only allow letters, numbers, hyphens, and underscores
        let valid_chars = name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_');
        if !valid_chars {
            return Err(TaskError::Validation(
                "Agent name can only contain letters, numbers, hyphens, and underscores".to_string()
            ));
        }

        Ok(())
    }

    /// Validate a task name
    /// 
    /// Task names must:
    /// - Be 1-200 characters long
    /// - Not be empty or only whitespace
    /// 
    /// # Arguments
    /// * `name` - The task name to validate
    /// 
    /// # Returns
    /// * `Ok(())` - If the name is valid
    /// * `Err(TaskError::Validation)` - If the name is invalid
    pub fn validate_task_name(name: &str) -> Result<()> {
        let trimmed = name.trim();
        
        if trimmed.is_empty() {
            return Err(TaskError::empty_field("name"));
        }

        if trimmed.len() > 200 {
            return Err(TaskError::Validation(
                "Task name must be at most 200 characters long".to_string()
            ));
        }

        Ok(())
    }

    /// Validate a task description
    /// 
    /// Task descriptions must:
    /// - Be 1-2000 characters long
    /// - Not be empty or only whitespace
    /// 
    /// # Arguments
    /// * `description` - The task description to validate
    /// 
    /// # Returns
    /// * `Ok(())` - If the description is valid
    /// * `Err(TaskError::Validation)` - If the description is invalid
    pub fn validate_task_description(description: &str) -> Result<()> {
        let trimmed = description.trim();
        
        if trimmed.is_empty() {
            return Err(TaskError::empty_field("description"));
        }

        if trimmed.len() > 2000 {
            return Err(TaskError::Validation(
                "Task description must be at most 2000 characters long".to_string()
            ));
        }

        Ok(())
    }

    /// Validate a complete NewTask structure
    /// 
    /// # Arguments
    /// * `task` - The new task to validate
    /// 
    /// # Returns
    /// * `Ok(())` - If the task is valid
    /// * `Err(TaskError::Validation)` - If any field is invalid
    pub fn validate_new_task(task: &NewTask) -> Result<()> {
        Self::validate_task_code(&task.code)?;
        Self::validate_task_name(&task.name)?;
        Self::validate_task_description(&task.description)?;
        Self::validate_agent_name(&task.owner_agent_name)?;
        Ok(())
    }

    /// Check if a state transition is valid for the given task
    /// 
    /// # Arguments
    /// * `task` - The current task
    /// * `new_state` - The desired new state
    /// 
    /// # Returns
    /// * `Ok(())` - If the transition is valid
    /// * `Err(TaskError::InvalidStateTransition)` - If the transition is invalid
    pub fn validate_state_transition(task: &Task, new_state: TaskState) -> Result<()> {
        if task.can_transition_to(new_state) {
            Ok(())
        } else {
            Err(TaskError::invalid_transition(task.state, new_state))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_valid_task_codes() {
        assert!(TaskValidator::validate_task_code("ARCH-01").is_ok());
        assert!(TaskValidator::validate_task_code("DB-15").is_ok());
        assert!(TaskValidator::validate_task_code("TEST123").is_ok());
        assert!(TaskValidator::validate_task_code("A1B2C3").is_ok());
        assert!(TaskValidator::validate_task_code("LONG-TASK-NAME").is_ok());
    }

    #[test]
    fn test_invalid_task_codes() {
        // Too short
        assert!(TaskValidator::validate_task_code("AB").is_err());
        
        // Too long
        assert!(TaskValidator::validate_task_code("A".repeat(21).as_str()).is_err());
        
        // Empty
        assert!(TaskValidator::validate_task_code("").is_err());
        
        // Starts with number
        assert!(TaskValidator::validate_task_code("1ABC").is_err());
        
        // Starts with hyphen
        assert!(TaskValidator::validate_task_code("-ABC").is_err());
        
        // Ends with hyphen
        assert!(TaskValidator::validate_task_code("ABC-").is_err());
        
        // Contains invalid characters
        assert!(TaskValidator::validate_task_code("ABC@123").is_err());
        assert!(TaskValidator::validate_task_code("ABC 123").is_err());
        
        // Consecutive hyphens
        assert!(TaskValidator::validate_task_code("ABC--123").is_err());
    }

    #[test]
    fn test_valid_agent_names() {
        assert!(TaskValidator::validate_agent_name("agent1").is_ok());
        assert!(TaskValidator::validate_agent_name("test-agent").is_ok());
        assert!(TaskValidator::validate_agent_name("agent_123").is_ok());
        assert!(TaskValidator::validate_agent_name("a").is_ok());
        assert!(TaskValidator::validate_agent_name("rust-architect").is_ok());
    }

    #[test]
    fn test_invalid_agent_names() {
        // Empty
        assert!(TaskValidator::validate_agent_name("").is_err());
        
        // Too long
        assert!(TaskValidator::validate_agent_name(&"a".repeat(51)).is_err());
        
        // Starts with hyphen
        assert!(TaskValidator::validate_agent_name("-agent").is_err());
        
        // Ends with hyphen
        assert!(TaskValidator::validate_agent_name("agent-").is_err());
        
        // Contains invalid characters
        assert!(TaskValidator::validate_agent_name("agent@123").is_err());
        assert!(TaskValidator::validate_agent_name("agent 123").is_err());
    }

    #[test]
    fn test_valid_task_names() {
        assert!(TaskValidator::validate_task_name("Simple task").is_ok());
        assert!(TaskValidator::validate_task_name("Task with symbols: !@#$%").is_ok());
        assert!(TaskValidator::validate_task_name("A").is_ok());
    }

    #[test]
    fn test_invalid_task_names() {
        // Empty
        assert!(TaskValidator::validate_task_name("").is_err());
        
        // Only whitespace
        assert!(TaskValidator::validate_task_name("   ").is_err());
        
        // Too long
        assert!(TaskValidator::validate_task_name(&"a".repeat(201)).is_err());
    }

    #[test]
    fn test_valid_task_descriptions() {
        assert!(TaskValidator::validate_task_description("A simple description").is_ok());
        assert!(TaskValidator::validate_task_description("A very long description with lots of details").is_ok());
    }

    #[test]
    fn test_invalid_task_descriptions() {
        // Empty
        assert!(TaskValidator::validate_task_description("").is_err());
        
        // Only whitespace
        assert!(TaskValidator::validate_task_description("   ").is_err());
        
        // Too long
        assert!(TaskValidator::validate_task_description(&"a".repeat(2001)).is_err());
    }

    #[test]
    fn test_validate_new_task() {
        let valid_task = NewTask {
            code: "ARCH-01".to_string(),
            name: "Architecture Task".to_string(),
            description: "Design the system architecture".to_string(),
            owner_agent_name: "rust-architect".to_string(),
        };
        
        assert!(TaskValidator::validate_new_task(&valid_task).is_ok());

        let invalid_task = NewTask {
            code: "".to_string(), // Invalid code
            name: "Architecture Task".to_string(),
            description: "Design the system architecture".to_string(),
            owner_agent_name: "rust-architect".to_string(),
        };
        
        assert!(TaskValidator::validate_new_task(&invalid_task).is_err());
    }

    #[test]
    fn test_validate_state_transition() {
        let task = Task {
            id: 1,
            code: "TEST-01".to_string(),
            name: "Test Task".to_string(),
            description: "Test description".to_string(),
            owner_agent_name: "test-agent".to_string(),
            state: TaskState::Created,
            inserted_at: Utc::now(),
            done_at: None,
        };

        // Valid transition
        assert!(TaskValidator::validate_state_transition(&task, TaskState::InProgress).is_ok());
        
        // Invalid transition
        assert!(TaskValidator::validate_state_transition(&task, TaskState::Done).is_err());
    }
}