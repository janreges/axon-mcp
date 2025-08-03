//! Enhanced Prompt Templates for 2025 AI Agent Best Practices
//! 
//! This module implements state-of-the-art prompt engineering techniques based on 2025
//! research for multi-agent coordination. It follows principles of:
//! - Clear task decomposition with structured contracts
//! - Lightweight communication design with explicit protocols
//! - Context management and memory scoping
//! - Dynamic effort scaling with micro-iterations
//! - Proper error handling and escalation paths

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::workspace_setup::{AiToolType, SuggestedAgent, ProjectArchetype};

/// Structured agent contract following 2025 best practices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContract {
    /// Agent role identifier
    pub role_name: String,
    /// Core mission statement
    pub mission: String,
    /// Concrete success criteria 
    pub success_criteria: Vec<String>,
    /// Expected deliverables with types and paths
    pub deliverables: Vec<Deliverable>,
    /// Tools and capabilities the agent can use
    pub tools_allowed: Vec<String>,
    /// Expected output format
    pub output_format: String,
    /// Escalation protocol for blockers
    pub escalation_protocol: String,
    /// Communication patterns with other agents
    pub communication_patterns: Vec<String>,
    /// Context scope limitations
    pub context_scope: Vec<String>,
}

/// Deliverable specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deliverable {
    /// Type of deliverable (code, doc, test, etc.)
    pub deliverable_type: String,
    /// Path or glob pattern for the deliverable
    pub path: String,
    /// Quality requirements
    pub quality_requirements: Vec<String>,
}

/// Capability schema following 2025 structured approach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDefinition {
    /// Capability identifier
    pub name: String,
    /// Required input artifacts
    pub inputs: Vec<String>,
    /// Expected output artifacts
    pub outputs: Vec<String>,
    /// Quality gates and validation criteria
    pub quality: Vec<String>,
    /// Dependencies on other capabilities
    pub depends_on: Vec<String>,
    /// Suggested libraries or tools
    pub suggested_tools: Vec<String>,
    /// Common patterns for this capability
    pub patterns: Vec<String>,
}

/// Coordination recipe for agent interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationRecipe {
    /// Recipe identifier
    pub name: String,
    /// Step-by-step coordination instructions
    pub steps: Vec<CoordinationStep>,
    /// Timeout settings for each step
    pub timeouts: HashMap<String, u32>,
    /// Error handling procedures
    pub error_handling: Vec<ErrorHandlingRule>,
}

/// Individual coordination step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationStep {
    /// Step identifier
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// MCP function to call
    pub mcp_function: String,
    /// Expected parameters
    pub parameters: serde_json::Value,
    /// Success criteria for this step
    pub success_criteria: String,
    /// What to do on failure
    pub failure_action: String,
}

/// Error handling rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandlingRule {
    /// Error condition to match
    pub condition: String,
    /// Action to take
    pub action: String,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Escalation target if retries fail
    pub escalation_target: String,
}

/// Enhanced prompt builder implementing 2025 best practices
#[derive(Clone)]
pub struct EnhancedPromptBuilder {
    /// Capability catalog
    capabilities: HashMap<String, CapabilityDefinition>,
    /// Coordination recipes
    coordination_recipes: HashMap<String, CoordinationRecipe>,
    /// Default configurations
    config: PromptBuilderConfig,
}

/// Configuration for prompt builder
#[derive(Debug, Clone)]
pub struct PromptBuilderConfig {
    /// Maximum context size for rolling context
    pub max_rolling_context: usize,
    /// Default timeout for coordination steps (seconds)
    pub default_timeout: u32,
    /// Maximum iterations per work session
    pub max_iterations: u32,
}

impl Default for PromptBuilderConfig {
    fn default() -> Self {
        Self {
            max_rolling_context: 4000,
            default_timeout: 60,
            max_iterations: 5,
        }
    }
}

impl EnhancedPromptBuilder {
    /// Create new prompt builder with default configuration
    pub fn new() -> Self {
        Self {
            capabilities: Self::default_capabilities(),
            coordination_recipes: Self::default_recipes(),
            config: PromptBuilderConfig::default(),
        }
    }

    /// Generate enhanced agent prompt following 2025 best practices
    pub fn generate_agent_prompt(
        &self,
        agent: &SuggestedAgent,
        archetype: &ProjectArchetype,
        project_context: &str,
        rolling_context: Option<&str>,
    ) -> String {
        let contract = self.create_agent_contract(agent, archetype);
        let coordination_recipe = self.get_coordination_recipe(archetype);
        
        format!(
            r#"[AGENT_CONTRACT_START]
{}
[AGENT_CONTRACT_END]

[STATIC_CONTEXT_START]
PROJECT: {project_context}
ARCHETYPE: {archetype}
[STATIC_CONTEXT_END]

{rolling_context_section}

=== YOUR ROLE AND MISSION ===
You are the '{agent_name}' agent with the mission: {mission}

Your success is measured by:
{success_criteria}

=== DELIVERABLES EXPECTED ===
{deliverables}

=== MCP COORDINATION PROTOCOL ===
You coordinate with other agents through these exact MCP function patterns:

{coordination_instructions}

=== WORK ITERATION PATTERN ===
Follow this micro-iteration pattern for dynamic effort scaling:

1. **Claim Work**: Use `claim_task(task_id, "{agent_name}")` to atomically claim your next task
2. **Start Session**: Use `start_work_session(task_id, "{agent_name}")` to begin time tracking
3. **Micro-Iteration Loop** (max {max_iterations} iterations):
   a. Analyze current requirements
   b. Generate/update deliverable
   c. Self-evaluate against success criteria
   d. If criteria met OR max iterations reached: proceed to step 4
   e. Otherwise: continue iteration with refined approach
4. **Handoff**: Use `create_task_message()` to share results with target agents
5. **Complete**: Use `set_task_state(task_id, "Done")` and `end_work_session(session_id)`

=== ERROR HANDLING & ESCALATION ===
{error_handling}

=== OUTPUT FORMAT ===
{output_format}

=== CONTEXT SCOPE ===
Focus only on: {context_scope}
Ignore information outside your scope to maintain efficiency.

Remember: You are part of a coordinated team. Always include clear context and next steps in your handoffs.
"#,
            serde_json::to_string_pretty(&contract).unwrap_or_else(|_| "Invalid contract".to_string()),
            project_context = project_context,
            archetype = archetype,
            rolling_context_section = rolling_context
                .map(|ctx| format!("[ROLLING_CONTEXT_START]\n{}\n[ROLLING_CONTEXT_END]\n", ctx))
                .unwrap_or_else(|| String::new()),
            agent_name = agent.name,
            mission = contract.mission,
            success_criteria = contract.success_criteria.iter()
                .enumerate()
                .map(|(i, criteria)| format!("{}. {}", i + 1, criteria))
                .collect::<Vec<_>>()
                .join("\n"),
            deliverables = contract.deliverables.iter()
                .map(|d| format!("- {} at {}: {}", d.deliverable_type, d.path, d.quality_requirements.join(", ")))
                .collect::<Vec<_>>()
                .join("\n"),
            coordination_instructions = self.format_coordination_instructions(&coordination_recipe),
            max_iterations = self.config.max_iterations,
            error_handling = self.format_error_handling(&coordination_recipe),
            output_format = contract.output_format,
            context_scope = contract.context_scope.join(", "),
        )
    }

    /// Create structured agent contract from suggested agent
    fn create_agent_contract(&self, agent: &SuggestedAgent, archetype: &ProjectArchetype) -> AgentContract {
        let (mission, success_criteria, deliverables, tools_allowed) = 
            self.get_archetype_specific_contract(agent, archetype);

        AgentContract {
            role_name: agent.name.clone(),
            mission,
            success_criteria,
            deliverables,
            tools_allowed,
            output_format: "Structured markdown with clear sections for analysis, implementation, and handoff details".to_string(),
            escalation_protocol: format!(
                "If blocked for >2 attempts OR >{}s: create_task_message(target='human-supervisor', type='blocker', content={{reason, attempts, next_best_step}})",
                self.config.default_timeout * 2
            ),
            communication_patterns: vec![
                "Always target specific agents in handoffs".to_string(),
                "Include context summary in every message".to_string(),
                "Use standard message types: handoff, question, blocker, solution".to_string(),
                "Acknowledge receipt of handoffs within 30s".to_string(),
            ],
            context_scope: agent.required_capabilities.clone(),
        }
    }

    /// Get archetype-specific contract details
    fn get_archetype_specific_contract(
        &self,
        agent: &SuggestedAgent,
        archetype: &ProjectArchetype,
    ) -> (String, Vec<String>, Vec<Deliverable>, Vec<String>) {
        match archetype {
            ProjectArchetype::CliTool if agent.name.contains("cli-developer") => (
                "Deliver a robust, user-friendly command-line interface that handles all specified requirements".to_string(),
                vec![
                    "Binary passes all tests with 0 clippy warnings".to_string(),
                    "Help text is comprehensive and auto-generated".to_string(),
                    "Error messages are clear and actionable".to_string(),
                    "Argument parsing handles edge cases gracefully".to_string(),
                ],
                vec![
                    Deliverable {
                        deliverable_type: "code".to_string(),
                        path: "src/bin/**/*.rs".to_string(),
                        quality_requirements: vec!["clippy clean".to_string(), "documented".to_string()],
                    },
                    Deliverable {
                        deliverable_type: "test".to_string(),
                        path: "tests/cli/**/*.rs".to_string(),
                        quality_requirements: vec!["coverage >90%".to_string()],
                    },
                ],
                vec!["cargo".to_string(), "clippy".to_string(), "rustfmt".to_string()],
            ),
            ProjectArchetype::WebApplication if agent.name.contains("backend-developer") => (
                "Implement scalable backend services with proper API design and data persistence".to_string(),
                vec![
                    "All API endpoints documented with OpenAPI/Swagger".to_string(),
                    "Database queries optimized with proper indexing".to_string(),
                    "Error handling follows RFC 7807 problem details".to_string(),
                    "Authentication and authorization properly implemented".to_string(),
                ],
                vec![
                    Deliverable {
                        deliverable_type: "code".to_string(),
                        path: "src/api/**/*.rs".to_string(),
                        quality_requirements: vec!["documented".to_string(), "tested".to_string()],
                    },
                    Deliverable {
                        deliverable_type: "schema".to_string(),
                        path: "migrations/**/*.sql".to_string(),
                        quality_requirements: vec!["peer reviewed".to_string()],
                    },
                ],
                vec!["sqlx".to_string(), "diesel".to_string(), "actix-web".to_string()],
            ),
            _ => (
                format!("Execute {} responsibilities within the {} project context", agent.description, archetype),
                vec![
                    "Deliverables meet project quality standards".to_string(),
                    "Coordination with dependent agents is seamless".to_string(),
                    "Documentation is clear and actionable".to_string(),
                ],
                vec![
                    Deliverable {
                        deliverable_type: "general".to_string(),
                        path: "output/**/*".to_string(),
                        quality_requirements: vec!["quality checked".to_string()],
                    },
                ],
                vec!["standard-tools".to_string()],
            ),
        }
    }

    /// Get coordination recipe for archetype
    fn get_coordination_recipe(&self, archetype: &ProjectArchetype) -> CoordinationRecipe {
        let recipe_name = match archetype {
            ProjectArchetype::CliTool => "cli-development",
            ProjectArchetype::WebApplication => "web-development",
            ProjectArchetype::ApiService => "api-development",
            _ => "general-development",
        };

        self.coordination_recipes
            .get(recipe_name)
            .cloned()
            .unwrap_or_else(|| self.default_coordination_recipe())
    }

    /// Format coordination instructions
    fn format_coordination_instructions(&self, recipe: &CoordinationRecipe) -> String {
        recipe.steps.iter()
            .map(|step| format!(
                "- **{}**: {} using `{}`\n  Success: {}\n  On failure: {}",
                step.description,
                step.description,
                step.mcp_function,
                step.success_criteria,
                step.failure_action
            ))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Format error handling instructions
    fn format_error_handling(&self, recipe: &CoordinationRecipe) -> String {
        recipe.error_handling.iter()
            .map(|rule| format!(
                "- **{}**: {} (max {} retries, escalate to {})",
                rule.condition,
                rule.action,
                rule.max_retries,
                rule.escalation_target
            ))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Default capability definitions following 2025 structured approach
    fn default_capabilities() -> HashMap<String, CapabilityDefinition> {
        let mut capabilities = HashMap::new();

        capabilities.insert("cli-development".to_string(), CapabilityDefinition {
            name: "cli-development".to_string(),
            inputs: vec!["cli_requirements.md".to_string(), "argument_specs.yaml".to_string()],
            outputs: vec!["src/bin/**/*.rs".to_string(), "tests/cli/**/*.rs".to_string()],
            quality: vec!["clippy==0".to_string(), "coverage>=90%".to_string(), "help_text_complete".to_string()],
            depends_on: vec!["rust-build".to_string()],
            suggested_tools: vec!["clap".to_string(), "structopt".to_string()],
            patterns: vec!["subcommands".to_string(), "argument_groups".to_string()],
        });

        capabilities.insert("api-design".to_string(), CapabilityDefinition {
            name: "api-design".to_string(),
            inputs: vec!["api_requirements.md".to_string(), "data_models.yaml".to_string()],
            outputs: vec!["openapi.yaml".to_string(), "src/api/**/*.rs".to_string()],
            quality: vec!["openapi_valid".to_string(), "documented".to_string(), "tested".to_string()],
            depends_on: vec!["data-modeling".to_string()],
            suggested_tools: vec!["actix-web".to_string(), "warp".to_string(), "axum".to_string()],
            patterns: vec!["rest".to_string(), "graphql".to_string(), "rpc".to_string()],
        });

        capabilities.insert("testing".to_string(), CapabilityDefinition {
            name: "testing".to_string(),
            inputs: vec!["src/**/*.rs".to_string(), "test_specs.md".to_string()],
            outputs: vec!["tests/**/*.rs".to_string(), "coverage_report.html".to_string()],
            quality: vec!["coverage>=85%".to_string(), "all_tests_pass".to_string()],
            depends_on: vec![],
            suggested_tools: vec!["cargo-test".to_string(), "proptest".to_string(), "mockall".to_string()],
            patterns: vec!["unit_tests".to_string(), "integration_tests".to_string(), "property_tests".to_string()],
        });

        capabilities
    }

    /// Default coordination recipes
    fn default_recipes() -> HashMap<String, CoordinationRecipe> {
        let mut recipes = HashMap::new();

        recipes.insert("cli-development".to_string(), CoordinationRecipe {
            name: "cli-development".to_string(),
            steps: vec![
                CoordinationStep {
                    id: "discover_tasks".to_string(),
                    description: "Find assigned CLI development tasks".to_string(),
                    mcp_function: "list_tasks".to_string(),
                    parameters: serde_json::json!({"owner": "cli-developer", "state": "Created"}),
                    success_criteria: "Tasks found and prioritized".to_string(),
                    failure_action: "Request task assignment from project manager".to_string(),
                },
                CoordinationStep {
                    id: "claim_task".to_string(),
                    description: "Atomically claim highest priority task".to_string(),
                    mcp_function: "claim_task".to_string(),
                    parameters: serde_json::json!({"task_id": "{{task_id}}", "agent_name": "cli-developer"}),
                    success_criteria: "Task successfully claimed".to_string(),
                    failure_action: "Try next available task".to_string(),
                },
                CoordinationStep {
                    id: "handoff_implementation".to_string(),
                    description: "Send completed implementation to QA".to_string(),
                    mcp_function: "create_task_message".to_string(),
                    parameters: serde_json::json!({
                        "task_code": "{{task_code}}",
                        "author_agent_name": "cli-developer",
                        "target_agent_name": "qa-tester",
                        "message_type": "handoff",
                        "content": "{{implementation_summary}}"
                    }),
                    success_criteria: "QA acknowledges handoff within 60s".to_string(),
                    failure_action: "Escalate to project manager".to_string(),
                },
            ],
            timeouts: {
                let mut timeouts = HashMap::new();
                timeouts.insert("discover_tasks".to_string(), 30);
                timeouts.insert("claim_task".to_string(), 10);
                timeouts.insert("handoff_implementation".to_string(), 90);
                timeouts
            },
            error_handling: vec![
                ErrorHandlingRule {
                    condition: "No tasks available".to_string(),
                    action: "Request work from project manager".to_string(),
                    max_retries: 2,
                    escalation_target: "human-supervisor".to_string(),
                },
                ErrorHandlingRule {
                    condition: "Implementation fails tests".to_string(),
                    action: "Analyze failure and retry implementation".to_string(),
                    max_retries: 3,
                    escalation_target: "senior-developer".to_string(),
                },
            ],
        });

        recipes
    }

    /// Default coordination recipe fallback
    fn default_coordination_recipe(&self) -> CoordinationRecipe {
        CoordinationRecipe {
            name: "general-development".to_string(),
            steps: vec![
                CoordinationStep {
                    id: "discover_work".to_string(),
                    description: "Find available work matching capabilities".to_string(),
                    mcp_function: "discover_work".to_string(),
                    parameters: serde_json::json!({
                        "agent_name": "{{agent_name}}",
                        "capabilities": "{{capabilities}}",
                        "max_tasks": 3
                    }),
                    success_criteria: "Relevant tasks identified".to_string(),
                    failure_action: "Request assignment from coordinator".to_string(),
                },
            ],
            timeouts: HashMap::new(),
            error_handling: vec![
                ErrorHandlingRule {
                    condition: "General error".to_string(),
                    action: "Log error and retry".to_string(),
                    max_retries: 2,
                    escalation_target: "human-supervisor".to_string(),
                },
            ],
        }
    }
}

impl Default for EnhancedPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate enhanced setup instructions following 2025 patterns
pub fn generate_enhanced_setup_instructions(ai_tool_type: AiToolType) -> String {
    format!(
        r#"# Enhanced AI Workspace Setup Instructions (2025 Edition)

## Overview
This setup follows 2025 best practices for multi-agent coordination using structured contracts, 
lightweight communication, and dynamic effort scaling.

## Setup Process

### Phase 1: Environment Preparation
1. **Initialize MCP Connection**
   ```bash
   # Ensure Axon MCP server is running
   curl http://localhost:3000/health
   ```

2. **Verify Capability Catalog**
   ```bash
   # Check that capability definitions are loaded
   curl http://localhost:3000/capabilities
   ```

### Phase 2: Agent Contract Generation
Use the enhanced prompt builder to generate agent contracts:

1. **Analyze PRD**: Extract project requirements and classify archetype
2. **Generate Contracts**: Create structured agent contracts with:
   - Clear mission statements
   - Concrete success criteria  
   - Defined deliverables
   - Communication protocols
   - Escalation procedures

### Phase 3: Coordination Setup
1. **Deploy Coordination Recipes**: Install archetype-specific coordination patterns
2. **Configure Error Handling**: Set up escalation chains and retry policies
3. **Initialize Context Management**: Configure static and rolling context boundaries

### Phase 4: Validation
1. **Contract Validation**: Ensure all agent contracts are structurally sound
2. **Communication Testing**: Verify agent-to-agent message flow
3. **Escalation Testing**: Confirm error handling paths work correctly

## Advanced Features (2025)

### Dynamic Effort Scaling
Agents automatically adjust iteration count based on:
- Task complexity analysis
- Available time budget
- Quality requirements
- Dependency constraints

### Lightweight Communication
- Targeted messaging prevents information overload
- Message threading maintains conversation context
- Automatic acknowledgment tracking ensures reliable handoffs

### Context Scoping
- Static context: Unchanging project requirements and contracts
- Rolling context: Recent conversation history and state updates
- Capability context: Agent-specific domain knowledge boundaries

## Tool-Specific Configuration

### Claude Code ({ai_tool_type})
- Main file: CLAUDE.md with enhanced agent contracts
- Agent directory: .claude/agents/ with individual contract files
- Coordination recipes: .claude/recipes/ with archetype-specific patterns

## Troubleshooting

### Common Issues
1. **Agent Confusion**: Usually indicates unclear success criteria or context scope
2. **Coordination Loops**: Often caused by missing acknowledgment patterns
3. **Escalation Failures**: Check that human-supervisor is properly configured

### Debug Commands
```bash
# Check agent contract validity
axon validate-contracts --workspace .

# Test coordination flow
axon test-coordination --scenario cli-development

# Analyze communication patterns  
axon analyze-messages --task-code TASK-001
```

This enhanced setup ensures your AI agents follow 2025 best practices for effective multi-agent coordination.
"#,
        ai_tool_type = ai_tool_type
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_setup::ProjectArchetype;

    #[test]
    fn test_enhanced_prompt_builder() {
        let builder = EnhancedPromptBuilder::new();
        let agent = SuggestedAgent {
            name: "cli-developer".to_string(),
            description: "Develops CLI tools".to_string(),
            required_capabilities: vec!["cli-development".to_string()],
            workload_percentage: 70.0,
            depends_on: vec![],
        };

        let prompt = builder.generate_agent_prompt(
            &agent,
            &ProjectArchetype::CliTool,
            "Markdown converter CLI tool",
            None,
        );

        assert!(prompt.contains("AGENT_CONTRACT_START"));
        assert!(prompt.contains("cli-developer"));
        assert!(prompt.contains("MCP COORDINATION PROTOCOL"));
        assert!(prompt.contains("ERROR HANDLING & ESCALATION"));
    }

    #[test]
    fn test_capability_definitions() {
        let capabilities = EnhancedPromptBuilder::default_capabilities();
        
        assert!(capabilities.contains_key("cli-development"));
        assert!(capabilities.contains_key("api-design"));
        assert!(capabilities.contains_key("testing"));

        let cli_cap = &capabilities["cli-development"];
        assert!(!cli_cap.inputs.is_empty());
        assert!(!cli_cap.outputs.is_empty());
        assert!(!cli_cap.quality.is_empty());
    }

    #[test]
    fn test_coordination_recipes() {
        let recipes = EnhancedPromptBuilder::default_recipes();
        
        assert!(recipes.contains_key("cli-development"));
        
        let cli_recipe = &recipes["cli-development"];
        assert!(!cli_recipe.steps.is_empty());
        assert!(!cli_recipe.error_handling.is_empty());
    }
}