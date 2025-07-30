use crate::error::TaskError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Circuit breaker for managing task failures in MCP v2 multi-agent system
/// 
/// This implementation is optimized for local AI agents with different failure patterns
/// compared to human workers or distributed systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    /// Maximum failures before quarantine (per failure type)
    failure_thresholds: HashMap<FailureType, i32>,
    /// Current failure counts
    failure_counts: HashMap<FailureType, i32>,
    /// Circuit breaker state
    state: CircuitState,
    /// Last failure timestamp
    last_failure: Option<chrono::DateTime<chrono::Utc>>,
}

/// Types of failures that can occur with AI agents
/// 
/// Different failure types have different implications:
/// - Capability mismatches should immediately reassign, not count against circuit breaker
/// - Context overflow requires task simplification
/// - Logic errors count toward circuit breaker and need investigation
/// - Environmental issues should retry with backoff
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureType {
    /// Agent lacks required capabilities - should reassign immediately
    CapabilityMismatch,
    /// Agent ran out of context window - reduce task complexity
    ContextOverflow,
    /// Agent made reasoning/logic errors - counts toward circuit breaker
    LogicError,
    /// External dependencies failed - retry with backoff
    Environmental,
    /// Invalid task requirements - needs human review
    InvalidRequirements,
    /// Agent gave inconsistent outputs - may indicate training issues
    InconsistentOutput,
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Normal operation
    Closed,
    /// Too many failures, task quarantined
    Open,
    /// Testing if task can be attempted again
    HalfOpen,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        let mut failure_thresholds = HashMap::new();
        
        // Set different thresholds for different failure types
        failure_thresholds.insert(FailureType::CapabilityMismatch, 1);  // Immediate reassignment
        failure_thresholds.insert(FailureType::ContextOverflow, 2);     // 2 attempts before simplification
        failure_thresholds.insert(FailureType::LogicError, 3);          // 3 attempts before quarantine
        failure_thresholds.insert(FailureType::Environmental, 5);       // 5 retries for transient issues
        failure_thresholds.insert(FailureType::InvalidRequirements, 1); // Immediate human review
        failure_thresholds.insert(FailureType::InconsistentOutput, 2);  // 2 attempts before investigation
        
        Self {
            failure_thresholds,
            failure_counts: HashMap::new(),
            state: CircuitState::Closed,
            last_failure: None,
        }
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker with custom thresholds
    pub fn new(failure_thresholds: HashMap<FailureType, i32>) -> Self {
        Self {
            failure_thresholds,
            failure_counts: HashMap::new(),
            state: CircuitState::Closed,
            last_failure: None,
        }
    }
    
    /// Record a failure and update circuit breaker state
    pub fn record_failure(&mut self, failure_type: FailureType) -> CircuitBreakerAction {
        self.last_failure = Some(chrono::Utc::now());
        
        // Increment failure count for this type
        let count = self.failure_counts.entry(failure_type).or_insert(0);
        *count += 1;
        
        // Store count value to avoid borrow checker issues
        let current_count = *count;
        
        // Check if threshold exceeded for this failure type
        let threshold = self.failure_thresholds.get(&failure_type).copied().unwrap_or(3);
        
        if current_count >= threshold {
            self.state = CircuitState::Open;
            self.determine_action(failure_type)
        } else {
            CircuitBreakerAction::Retry { 
                delay_seconds: self.calculate_backoff(failure_type, current_count),
                suggestion: self.get_retry_suggestion(failure_type),
            }
        }
    }
    
    /// Record a successful task completion
    pub fn record_success(&mut self) {
        // Reset failure counts on success
        self.failure_counts.clear();
        self.state = CircuitState::Closed;
        self.last_failure = None;
    }
    
    /// Check if task can be attempted
    pub fn can_attempt(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true,
        }
    }
    
    /// Get current state
    pub fn state(&self) -> CircuitState {
        self.state
    }
    
    /// Get failure counts
    pub fn failure_counts(&self) -> &HashMap<FailureType, i32> {
        &self.failure_counts
    }
    
    /// Try to reset circuit breaker (requires manual intervention for some failure types)
    pub fn try_reset(&mut self, authorized_by: &str) -> Result<(), TaskError> {
        match self.state {
            CircuitState::Open => {
                // Check if enough time has passed for automatic reset
                if let Some(last_failure) = self.last_failure {
                    let elapsed = chrono::Utc::now() - last_failure;
                    
                    // Allow automatic reset after 1 hour for environmental issues
                    if elapsed.num_hours() >= 1 && self.is_transient_failure_only() {
                        self.state = CircuitState::HalfOpen;
                        Ok(())
                    } else {
                        // Require manual authorization for non-transient failures
                        if authorized_by.is_empty() {
                            Err(TaskError::CircuitBreakerOpen(
                                "Manual authorization required to reset circuit breaker".to_string()
                            ))
                        } else {
                            self.state = CircuitState::HalfOpen;
                            self.failure_counts.clear();
                            Ok(())
                        }
                    }
                } else {
                    self.state = CircuitState::HalfOpen;
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
    
    /// Calculate exponential backoff delay
    fn calculate_backoff(&self, failure_type: FailureType, attempt: i32) -> u64 {
        match failure_type {
            FailureType::CapabilityMismatch => 0,  // Immediate reassignment
            FailureType::ContextOverflow => 30,    // 30 seconds to allow for cleanup
            FailureType::LogicError => 60 * attempt as u64, // 1, 2, 3 minutes
            FailureType::Environmental => (2_u64.pow(attempt as u32 - 1)) * 60, // Exponential backoff
            FailureType::InvalidRequirements => 0, // Immediate human review
            FailureType::InconsistentOutput => 120, // 2 minutes for investigation
        }
    }
    
    /// Get retry suggestion based on failure type
    fn get_retry_suggestion(&self, failure_type: FailureType) -> String {
        match failure_type {
            FailureType::CapabilityMismatch => 
                "Reassign to agent with matching capabilities".to_string(),
            FailureType::ContextOverflow => 
                "Reduce task complexity or break into smaller subtasks".to_string(),
            FailureType::LogicError => 
                "Review task requirements and provide additional context".to_string(),
            FailureType::Environmental => 
                "Check external dependencies and network connectivity".to_string(),
            FailureType::InvalidRequirements => 
                "Task requirements need human review and clarification".to_string(),
            FailureType::InconsistentOutput => 
                "Agent may need reinitialization or different approach".to_string(),
        }
    }
    
    /// Determine action based on failure type and circuit breaker state
    fn determine_action(&self, failure_type: FailureType) -> CircuitBreakerAction {
        match failure_type {
            FailureType::CapabilityMismatch => CircuitBreakerAction::Reassign {
                reason: "Agent lacks required capabilities".to_string(),
                required_capabilities: vec![], // Would be filled by caller
            },
            FailureType::ContextOverflow => CircuitBreakerAction::Simplify {
                reason: "Task exceeds agent context window".to_string(),
                suggestion: "Break task into smaller, sequential subtasks".to_string(),
            },
            FailureType::InvalidRequirements => CircuitBreakerAction::HumanReview {
                reason: "Task requirements are unclear or invalid".to_string(),
                escalation_level: "manager".to_string(),
            },
            _ => CircuitBreakerAction::Quarantine {
                reason: format!("Too many {} failures", self.format_failure_type(failure_type)),
                retry_after: chrono::Utc::now() + chrono::Duration::hours(1),
            },
        }
    }
    
    /// Check if only transient failures have occurred
    fn is_transient_failure_only(&self) -> bool {
        self.failure_counts.keys().all(|&failure_type| {
            matches!(failure_type, FailureType::Environmental)
        })
    }
    
    /// Format failure type for display
    fn format_failure_type(&self, failure_type: FailureType) -> &'static str {
        match failure_type {
            FailureType::CapabilityMismatch => "capability mismatch",
            FailureType::ContextOverflow => "context overflow",
            FailureType::LogicError => "logic error",
            FailureType::Environmental => "environmental",
            FailureType::InvalidRequirements => "invalid requirements",
            FailureType::InconsistentOutput => "inconsistent output",
        }
    }
}

/// Actions that can be taken when circuit breaker is triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CircuitBreakerAction {
    /// Retry the task after a delay
    Retry {
        delay_seconds: u64,
        suggestion: String,
    },
    /// Reassign task to different agent
    Reassign {
        reason: String,
        required_capabilities: Vec<String>,
    },
    /// Simplify or break down the task
    Simplify {
        reason: String,
        suggestion: String,
    },
    /// Quarantine task for later review
    Quarantine {
        reason: String,
        retry_after: chrono::DateTime<chrono::Utc>,
    },
    /// Escalate to human review
    HumanReview {
        reason: String,
        escalation_level: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_default_thresholds() {
        let cb = CircuitBreaker::default();
        
        // Should start in closed state
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_attempt());
    }

    #[test]
    fn test_capability_mismatch_immediate_reassignment() {
        let mut cb = CircuitBreaker::default();
        
        let action = cb.record_failure(FailureType::CapabilityMismatch);
        
        // Should trigger immediate reassignment
        assert_eq!(cb.state(), CircuitState::Open);
        match action {
            CircuitBreakerAction::Reassign { .. } => {},
            _ => panic!("Expected reassignment action"),
        }
    }

    #[test]
    fn test_logic_error_progressive_failures() {
        let mut cb = CircuitBreaker::default();
        
        // First two failures should retry
        let action1 = cb.record_failure(FailureType::LogicError);
        match action1 {
            CircuitBreakerAction::Retry { .. } => {},
            _ => panic!("Expected retry action"),
        }
        assert_eq!(cb.state(), CircuitState::Closed);
        
        let action2 = cb.record_failure(FailureType::LogicError);
        match action2 {
            CircuitBreakerAction::Retry { .. } => {},
            _ => panic!("Expected retry action"),
        }
        assert_eq!(cb.state(), CircuitState::Closed);
        
        // Third failure should open circuit
        let action3 = cb.record_failure(FailureType::LogicError);
        match action3 {
            CircuitBreakerAction::Quarantine { .. } => {},
            _ => panic!("Expected quarantine action"),
        }
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_success_resets_failures() {
        let mut cb = CircuitBreaker::default();
        
        // Record some failures
        cb.record_failure(FailureType::LogicError);
        cb.record_failure(FailureType::LogicError);
        
        assert!(!cb.failure_counts.is_empty());
        
        // Success should reset everything
        cb.record_success();
        
        assert!(cb.failure_counts.is_empty());
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_environmental_failures_allow_automatic_reset() {
        let mut cb = CircuitBreaker::default();
        
        // Trigger environmental failures to open circuit
        for _ in 0..5 {
            cb.record_failure(FailureType::Environmental);
        }
        
        assert_eq!(cb.state(), CircuitState::Open);
        
        // Should allow automatic reset for transient failures
        assert!(cb.is_transient_failure_only());
    }
}