/// AI Tool Adapter Pattern for Multi-Tool Support
/// 
/// This module provides an extensible adapter pattern for supporting different AI tools
/// in Axon workspace setup automation. Currently supports Claude Code with planned
/// support for AutoGen and CrewAI.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

use crate::workspace_setup::{
    AiToolType, PrdDocument, AgenticWorkflowDescription, 
    SetupInstructions, MainAiFileInstructions, MainAiFileData,
    WorkspaceManifest, AgentRegistration,
};
use crate::prompt_templates::EnhancedPromptBuilder;
use crate::error::{TaskError, Result};

/// Trait defining the interface that all AI tool adapters must implement
#[async_trait]
pub trait AiToolAdapter: Send + Sync {
    /// Get the AI tool type this adapter supports
    fn tool_type(&self) -> AiToolType;
    
    /// Get setup instructions specific to this AI tool
    async fn get_setup_instructions(&self) -> Result<SetupInstructions>;
    
    /// Get instructions for creating the main coordination file
    async fn get_main_file_instructions(&self) -> Result<MainAiFileInstructions>;
    
    /// Create the main coordination file with tool-specific content
    async fn create_main_file(
        &self, 
        content: &str, 
        project_name: Option<&str>,
        overwrite_existing: bool
    ) -> Result<MainAiFileData>;
    
    /// Generate agent definition files for this tool
    async fn generate_agent_files(
        &self,
        agents: &[AgentRegistration],
        output_dir: &str
    ) -> Result<Vec<String>>;
    
    /// Create tool-specific workspace structure (directories, config files)
    async fn create_workspace_structure(&self, output_dir: &str) -> Result<()>;
    
    /// Generate workspace manifest with tool-specific metadata
    async fn generate_manifest(
        &self,
        prd: &PrdDocument,
        workflow: &AgenticWorkflowDescription,
        include_generated_files: bool
    ) -> Result<WorkspaceManifest>;
    
    /// Validate that a workspace is properly configured for this tool
    async fn validate_workspace(&self, workspace_dir: &str) -> Result<ValidationResult>;
}

/// Result of workspace validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the workspace is valid for this AI tool
    pub is_valid: bool,
    /// List of validation issues found
    pub issues: Vec<ValidationIssue>,
    /// Recommendations for fixing issues
    pub recommendations: Vec<String>,
}

/// Individual validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Issue severity level
    pub severity: ValidationSeverity,
    /// Description of the issue
    pub description: String,
    /// File or component where issue was found
    pub location: Option<String>,
}

/// Severity levels for validation issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Critical issue that prevents workspace from functioning
    Critical,
    /// Important issue that may cause problems
    Warning,
    /// Minor issue or suggestion for improvement
    Info,
}

/// Registry for managing AI tool adapters
pub struct AiToolAdapterRegistry {
    adapters: HashMap<AiToolType, Box<dyn AiToolAdapter>>,
}

impl AiToolAdapterRegistry {
    /// Create a new adapter registry with default adapters
    pub fn new() -> Self {
        let mut registry = Self {
            adapters: HashMap::new(),
        };
        
        // Register built-in adapters
        registry.register(Box::new(ClaudeCodeAdapter::new()));
        
        registry
    }
    
    /// Register a new adapter for an AI tool type
    pub fn register(&mut self, adapter: Box<dyn AiToolAdapter>) {
        let tool_type = adapter.tool_type();
        self.adapters.insert(tool_type, adapter);
    }
    
    /// Get an adapter for a specific AI tool type
    pub fn get_adapter(&self, tool_type: AiToolType) -> Result<&dyn AiToolAdapter> {
        self.adapters
            .get(&tool_type)
            .map(|adapter| adapter.as_ref())
            .ok_or_else(|| TaskError::UnsupportedAiTool(tool_type.to_string()))
    }
    
    /// Get list of supported AI tool types
    pub fn supported_tools(&self) -> Vec<AiToolType> {
        self.adapters.keys().cloned().collect()
    }
    
    /// Check if a tool type is supported
    pub fn is_supported(&self, tool_type: AiToolType) -> bool {
        self.adapters.contains_key(&tool_type)
    }
}

impl Default for AiToolAdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Claude Code adapter implementation
pub struct ClaudeCodeAdapter {
    /// Adapter configuration
    config: ClaudeCodeConfig,
    /// Enhanced prompt builder for 2025 best practices
    prompt_builder: EnhancedPromptBuilder,
}

/// Configuration for Claude Code adapter
#[derive(Debug, Clone)]
pub struct ClaudeCodeConfig {
    /// Main coordination file name
    pub main_file_name: String,
    /// Agent directory path
    pub agents_dir: String,
    /// Commands directory path  
    pub commands_dir: String,
    /// Manifest directory path
    pub manifest_dir: String,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        Self {
            main_file_name: "CLAUDE.md".to_string(),
            agents_dir: ".claude/agents".to_string(),
            commands_dir: ".claude/commands".to_string(),
            manifest_dir: ".axon".to_string(),
        }
    }
}

impl ClaudeCodeAdapter {
    /// Create a new Claude Code adapter with default configuration
    pub fn new() -> Self {
        Self {
            config: ClaudeCodeConfig::default(),
            prompt_builder: EnhancedPromptBuilder::new(),
        }
    }
    
    /// Create a new Claude Code adapter with custom configuration
    pub fn with_config(config: ClaudeCodeConfig) -> Self {
        Self { 
            config,
            prompt_builder: EnhancedPromptBuilder::new(),
        }
    }
}

#[async_trait]
impl AiToolAdapter for ClaudeCodeAdapter {
    fn tool_type(&self) -> AiToolType {
        AiToolType::ClaudeCode
    }
    
    async fn get_setup_instructions(&self) -> Result<SetupInstructions> {
        use crate::workspace_setup::{SetupStep, RequiredMcpFunction, ManifestTemplate};
        
        Ok(SetupInstructions {
            schema_version: "2.0".to_string(), // Updated for 2025 enhancements
            ai_tool_type: AiToolType::ClaudeCode,
            setup_steps: vec![
                SetupStep {
                    id: "verify-mcp-connection".to_string(),
                    name: "Verify MCP Connection".to_string(),
                    description: "Ensure Axon MCP server is running and accessible. Run: curl http://localhost:3000/health".to_string(),
                    order: 1,
                    required: true,
                    validation_script: Some("curl -s http://localhost:3000/health | grep -q \"healthy\"".to_string()),
                },
                SetupStep {
                    id: "create-directories".to_string(),
                    name: "Create enhanced workspace directories".to_string(),
                    description: format!("Set up the directory structure for 2025 enhanced Claude Code workspace. Run: mkdir -p {} {} {} {}/.claude/recipes {}/.claude/capabilities", 
                        self.config.agents_dir, self.config.commands_dir, self.config.manifest_dir, self.config.manifest_dir, self.config.manifest_dir),
                    order: 2,
                    required: true,
                    validation_script: None,
                },
                SetupStep {
                    id: "install-capability-catalog".to_string(),
                    name: "Install enhanced capability catalog".to_string(),
                    description: "Deploy 2025 capability definitions and coordination recipes".to_string(),
                    order: 3,
                    required: true,
                    validation_script: Some(format!("test -f {}/.claude/capabilities/catalog.json", self.config.manifest_dir)),
                },
                SetupStep {
                    id: "create-main-file".to_string(),
                    name: "Create enhanced coordination file".to_string(),
                    description: format!("Generate the {} file with 2025 coordination instructions", self.config.main_file_name),
                    order: 4,
                    required: true,
                    validation_script: Some(format!("test -f {}", self.config.main_file_name)),
                },
                SetupStep {
                    id: "generate-enhanced-agents".to_string(),
                    name: "Generate enhanced agent contracts".to_string(),
                    description: format!("Create agent definition files with structured contracts in {}", self.config.agents_dir),
                    order: 5,
                    required: true,
                    validation_script: Some(format!("ls {} | wc -l", self.config.agents_dir)),
                },
                SetupStep {
                    id: "validate-workspace".to_string(),
                    name: "Validate enhanced workspace".to_string(),
                    description: "Run comprehensive validation of 2025 workspace setup".to_string(),
                    order: 6,
                    required: true,
                    validation_script: Some("axon validate-workspace --enhanced".to_string()),
                },
            ],
            required_mcp_functions: vec![
                RequiredMcpFunction {
                    function_name: "get_agentic_workflow_description".to_string(),
                    when_to_call: "When analyzing PRD to generate enhanced agent recommendations".to_string(),
                    expected_parameters: "PRD content string, archetype classification, agent count".to_string(),
                },
                RequiredMcpFunction {
                    function_name: "create_task".to_string(),
                    when_to_call: "When agents need to create new tasks with structured metadata".to_string(),
                    expected_parameters: "Task code, name, description, owner, capabilities, priority".to_string(),
                },
                RequiredMcpFunction {
                    function_name: "claim_task".to_string(),
                    when_to_call: "When agents atomically claim tasks for execution".to_string(),
                    expected_parameters: "Task ID, agent name".to_string(),
                },
                RequiredMcpFunction {
                    function_name: "start_work_session".to_string(),
                    when_to_call: "When agents begin time tracking for dynamic effort scaling".to_string(),
                    expected_parameters: "Task ID, agent name".to_string(),
                },
                RequiredMcpFunction {
                    function_name: "create_task_message".to_string(),
                    when_to_call: "When agents communicate through lightweight protocols".to_string(),
                    expected_parameters: "Task code, author, target, message type (handoff/question/blocker), content".to_string(),
                },
                RequiredMcpFunction {
                    function_name: "end_work_session".to_string(),
                    when_to_call: "When agents complete work sessions with productivity metrics".to_string(),
                    expected_parameters: "Session ID, notes, productivity score".to_string(),
                },
            ],
            manifest_template: ManifestTemplate {
                target_path: format!("{}/.axon/manifest.json", self.config.manifest_dir),
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "schema_version": {"type": "string"},
                        "ai_tool_type": {"type": "string"},
                        "project": {"type": "object"},
                        "agents": {"type": "array"},
                        "workflow": {"type": "object"},
                        "enhanced_features": {
                            "type": "object",
                            "properties": {
                                "contracts": {"type": "array"},
                                "capabilities": {"type": "object"},
                                "coordination_recipes": {"type": "array"}
                            }
                        }
                    }
                }),
                example: serde_json::json!({
                    "schema_version": "2.0",
                    "ai_tool_type": "claude-code",
                    "project": {
                        "name": "Example Project",
                        "description": "Example description",
                        "archetype": "web-application"
                    },
                    "enhanced_features": {
                        "prompt_version": "2025.1",
                        "capability_catalog": true,
                        "structured_contracts": true,
                        "coordination_recipes": true
                    }
                }),
            },
        })
    }
    
    async fn get_main_file_instructions(&self) -> Result<MainAiFileInstructions> {
        use crate::workspace_setup::SectionTemplate;
        use std::collections::HashMap;
        
        Ok(MainAiFileInstructions {
            ai_tool_type: AiToolType::ClaudeCode,
            file_name: self.config.main_file_name.clone(),
            structure_template: vec![
                SectionTemplate {
                    id: "project-overview".to_string(),
                    title: "Project Overview".to_string(),
                    template: "# {{project_name}}\n\n{{project_description}}".to_string(),
                    order: 1,
                    required: true,
                    placeholders: {
                        let mut map = HashMap::new();
                        map.insert("project_name".to_string(), "Name of the project from PRD".to_string());
                        map.insert("project_description".to_string(), "Brief project description".to_string());
                        map
                    },
                },
                SectionTemplate {
                    id: "agent-coordination".to_string(),
                    title: "Agent Coordination".to_string(),
                    template: "## Agent Coordination\n\n{{coordination_instructions}}".to_string(),
                    order: 2,
                    required: true,
                    placeholders: {
                        let mut map = HashMap::new();
                        map.insert("coordination_instructions".to_string(), "Instructions for agent coordination through Axon MCP".to_string());
                        map
                    },
                },
            ],
            content_guidelines: vec![
                "Use clear, actionable language for AI agents".to_string(),
                "Include specific examples of expected inputs/outputs".to_string(),
                "Define coordination protocols between agents".to_string(),
                "Specify task handoff procedures".to_string(),
            ],
            examples: {
                let mut examples = HashMap::new();
                examples.insert("coordination_example".to_string(),
                    "1. Use list_tasks to find your assigned tasks\n2. Use claim_task to claim available work\n3. Use create_task_message for handoffs".to_string());
                examples
            },
        })
    }
    
    async fn create_main_file(
        &self,
        content: &str,
        project_name: Option<&str>,
        _overwrite_existing: bool
    ) -> Result<MainAiFileData> {
        use crate::workspace_setup::FileSection;
        
        // Parse content into sections (simplified implementation)
        let sections = vec![
            FileSection {
                title: "Project Overview".to_string(),
                content: project_name.unwrap_or("Project").to_string(),
                order: 1,
            },
            FileSection {
                title: "Agent Coordination".to_string(),
                content: "Agent coordination instructions".to_string(),
                order: 2,
            },
        ];
        
        Ok(MainAiFileData {
            ai_tool_type: AiToolType::ClaudeCode,
            file_name: self.config.main_file_name.clone(),
            content: content.to_string(),
            sections,
        })
    }
    
    async fn generate_agent_files(
        &self,
        agents: &[AgentRegistration],
        output_dir: &str
    ) -> Result<Vec<String>> {
        let mut generated_files = Vec::new();
        
        for agent in agents {
            // Generate enhanced prompt for this agent using 2025 best practices
            let enhanced_prompt = self.prompt_builder.generate_agent_prompt(
                &crate::workspace_setup::SuggestedAgent {
                    name: agent.name.clone(),
                    description: agent.description.clone(),
                    required_capabilities: agent.capabilities.clone(),
                    workload_percentage: 100.0 / agents.len() as f32,
                    depends_on: agent.dependencies.clone(),
                },
                &crate::workspace_setup::ProjectArchetype::Generic, // TODO: Determine from PRD
                "Project context from PRD analysis",
                None
            );
            
            // Create enhanced agent file content
            let _file_content = format!(
                r#"# Agent: {agent_name}

## Role Contract
{enhanced_prompt}

## Configuration
- **Name**: {agent_name}
- **Type**: {ai_tool_type}
- **Capabilities**: {capabilities}
- **Dependencies**: {dependencies}

## Usage
This agent should be instantiated with the above prompt as the system message.
The contract defines clear expectations, coordination protocols, and escalation procedures.

## 2025 Enhancements
- Structured contracts with measurable success criteria
- Lightweight communication protocols with MCP functions
- Dynamic effort scaling through micro-iterations
- Clear error handling and escalation procedures
- Context scoping to prevent information overload
"#,
                agent_name = agent.name,
                enhanced_prompt = enhanced_prompt,
                ai_tool_type = agent.ai_tool_type,
                capabilities = agent.capabilities.join(", "),
                dependencies = agent.dependencies.join(", "),
            );
            
            let file_path = format!("{}/{}/{}.md", output_dir, self.config.agents_dir, agent.name);
            
            // In a real implementation, we would write the file here
            // For now, we just track the file path and content would be written by the caller
            generated_files.push(file_path);
        }
        
        Ok(generated_files)
    }
    
    async fn create_workspace_structure(&self, _output_dir: &str) -> Result<()> {
        // This would create the actual directory structure
        // For now, we'll just return success as the CLI handles directory creation
        Ok(())
    }
    
    async fn generate_manifest(
        &self,
        prd: &PrdDocument,
        workflow: &AgenticWorkflowDescription,
        include_generated_files: bool
    ) -> Result<WorkspaceManifest> {
        use crate::workspace_setup::{ProjectMetadata, GeneratedFile};
        use chrono::Utc;
        
        let agents = workflow.suggested_agents.iter().map(|agent| {
            // Generate enhanced prompt for each agent
            let enhanced_prompt = self.prompt_builder.generate_agent_prompt(
                agent,
                &crate::workspace_setup::ProjectArchetype::Generic, // TODO: Pass actual archetype
                &format!("Project: {}", prd.title),
                None
            );
            
            AgentRegistration {
                name: agent.name.clone(),
                description: agent.description.clone(),
                prompt: enhanced_prompt,  // Use enhanced prompt instead of basic template
                capabilities: agent.required_capabilities.clone(),
                ai_tool_type: AiToolType::ClaudeCode,
                dependencies: agent.depends_on.clone(),
            }
        }).collect();
        
        let generated_files = if include_generated_files {
            vec![
                GeneratedFile {
                    path: self.config.main_file_name.clone(),
                    file_type: "coordination".to_string(),
                    description: "Main coordination file for Claude Code".to_string(),
                    critical: true,
                },
            ]
        } else {
            vec![]
        };
        
        Ok(WorkspaceManifest {
            schema_version: "1.0".to_string(),
            ai_tool_type: AiToolType::ClaudeCode,
            project: ProjectMetadata {
                name: prd.title.clone(),
                description: prd.overview.clone().unwrap_or_else(|| "No description available".to_string()),
                complexity_score: 5, // Default complexity
                primary_domain: "software-development".to_string(),
                technologies: prd.technical_requirements.clone(),
            },
            agents,
            workflow: workflow.clone(),
            setup_instructions: vec![], // Could be populated from get_setup_instructions
            generated_files,
            created_at: Utc::now(),
            axon_version: "0.1.0".to_string(),
        })
    }
    
    async fn validate_workspace(&self, workspace_dir: &str) -> Result<ValidationResult> {
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        
        // Check if main file exists
        let main_file_path = format!("{}/{}", workspace_dir, self.config.main_file_name);
        if !std::path::Path::new(&main_file_path).exists() {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Critical,
                description: format!("Main coordination file {} is missing", self.config.main_file_name),
                location: Some(main_file_path),
            });
            recommendations.push(format!("Create {} with project coordination instructions", self.config.main_file_name));
        }
        
        // Check if agents directory exists
        let agents_dir_path = format!("{}/{}", workspace_dir, self.config.agents_dir);
        if !std::path::Path::new(&agents_dir_path).exists() {
            issues.push(ValidationIssue {
                severity: ValidationSeverity::Warning,
                description: "Agents directory is missing".to_string(),
                location: Some(agents_dir_path),
            });
            recommendations.push(format!("Create {} directory with agent definition files", self.config.agents_dir));
        }
        
        let is_valid = issues.iter().all(|issue| {
            !matches!(issue.severity, ValidationSeverity::Critical)
        });
        
        Ok(ValidationResult {
            is_valid,
            issues,
            recommendations,
        })
    }
}

/// Future: AutoGen adapter (placeholder implementation)
pub struct AutoGenAdapter;

#[async_trait]
impl AiToolAdapter for AutoGenAdapter {
    fn tool_type(&self) -> AiToolType {
        AiToolType::AutoGen
    }
    
    async fn get_setup_instructions(&self) -> Result<SetupInstructions> {
        Err(TaskError::UnsupportedOperation("AutoGen adapter not yet implemented".to_string()))
    }
    
    async fn get_main_file_instructions(&self) -> Result<MainAiFileInstructions> {
        Err(TaskError::UnsupportedOperation("AutoGen adapter not yet implemented".to_string()))
    }
    
    async fn create_main_file(&self, _content: &str, _project_name: Option<&str>, _overwrite_existing: bool) -> Result<MainAiFileData> {
        Err(TaskError::UnsupportedOperation("AutoGen adapter not yet implemented".to_string()))
    }
    
    async fn generate_agent_files(&self, _agents: &[AgentRegistration], _output_dir: &str) -> Result<Vec<String>> {
        Err(TaskError::UnsupportedOperation("AutoGen adapter not yet implemented".to_string()))
    }
    
    async fn create_workspace_structure(&self, _output_dir: &str) -> Result<()> {
        Err(TaskError::UnsupportedOperation("AutoGen adapter not yet implemented".to_string()))
    }
    
    async fn generate_manifest(&self, _prd: &PrdDocument, _workflow: &AgenticWorkflowDescription, _include_generated_files: bool) -> Result<WorkspaceManifest> {
        Err(TaskError::UnsupportedOperation("AutoGen adapter not yet implemented".to_string()))
    }
    
    async fn validate_workspace(&self, _workspace_dir: &str) -> Result<ValidationResult> {
        Err(TaskError::UnsupportedOperation("AutoGen adapter not yet implemented".to_string()))
    }
}

/// Future: CrewAI adapter (placeholder implementation)
pub struct CrewAiAdapter;

#[async_trait]
impl AiToolAdapter for CrewAiAdapter {
    fn tool_type(&self) -> AiToolType {
        AiToolType::CrewAi
    }
    
    async fn get_setup_instructions(&self) -> Result<SetupInstructions> {
        Err(TaskError::UnsupportedOperation("CrewAI adapter not yet implemented".to_string()))
    }
    
    async fn get_main_file_instructions(&self) -> Result<MainAiFileInstructions> {
        Err(TaskError::UnsupportedOperation("CrewAI adapter not yet implemented".to_string()))
    }
    
    async fn create_main_file(&self, _content: &str, _project_name: Option<&str>, _overwrite_existing: bool) -> Result<MainAiFileData> {
        Err(TaskError::UnsupportedOperation("CrewAI adapter not yet implemented".to_string()))
    }
    
    async fn generate_agent_files(&self, _agents: &[AgentRegistration], _output_dir: &str) -> Result<Vec<String>> {
        Err(TaskError::UnsupportedOperation("CrewAI adapter not yet implemented".to_string()))
    }
    
    async fn create_workspace_structure(&self, _output_dir: &str) -> Result<()> {
        Err(TaskError::UnsupportedOperation("CrewAI adapter not yet implemented".to_string()))
    }
    
    async fn generate_manifest(&self, _prd: &PrdDocument, _workflow: &AgenticWorkflowDescription, _include_generated_files: bool) -> Result<WorkspaceManifest> {
        Err(TaskError::UnsupportedOperation("CrewAI adapter not yet implemented".to_string()))
    }
    
    async fn validate_workspace(&self, _workspace_dir: &str) -> Result<ValidationResult> {
        Err(TaskError::UnsupportedOperation("CrewAI adapter not yet implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_adapter_registry() {
        let registry = AiToolAdapterRegistry::new();
        
        // Test supported tools
        assert!(registry.is_supported(AiToolType::ClaudeCode));
        assert!(!registry.is_supported(AiToolType::AutoGen));
        assert!(!registry.is_supported(AiToolType::CrewAi));
        
        // Test getting adapters
        assert!(registry.get_adapter(AiToolType::ClaudeCode).is_ok());
        assert!(registry.get_adapter(AiToolType::AutoGen).is_err());
    }
    
    #[tokio::test]
    async fn test_claude_code_adapter() {
        let adapter = ClaudeCodeAdapter::new();
        
        assert_eq!(adapter.tool_type(), AiToolType::ClaudeCode);
        
        // Test setup instructions
        let instructions = adapter.get_setup_instructions().await.unwrap();
        assert_eq!(instructions.ai_tool_type, AiToolType::ClaudeCode);
        assert!(!instructions.setup_steps.is_empty());
        
        // Test main file instructions
        let main_instructions = adapter.get_main_file_instructions().await.unwrap();
        assert_eq!(main_instructions.file_name, "CLAUDE.md");
        assert!(!main_instructions.structure_template.is_empty());
    }
    
    #[test]
    fn test_validation_result() {
        let result = ValidationResult {
            is_valid: false,
            issues: vec![
                ValidationIssue {
                    severity: ValidationSeverity::Critical,
                    description: "Critical issue".to_string(),
                    location: Some("file.md".to_string()),
                }
            ],
            recommendations: vec!["Fix the critical issue".to_string()],
        };
        
        assert!(!result.is_valid);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.recommendations.len(), 1);
    }
}