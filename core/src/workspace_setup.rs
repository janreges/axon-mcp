//! Workspace Setup Module
//!
//! This module provides functionality for automatically setting up AI agent workspaces
//! based on PRD (Product Requirements Document) analysis. It enables one-command workspace
//! initialization for AI tools like Claude Code.
//!
//! # Core Features
//!
//! - PRD analysis and parsing
//! - Agent role generation based on project requirements  
//! - Workspace manifest creation
//! - AI tool-specific file generation
//! - Template-driven prompt engineering
//!
//! # Example Usage
//!
//! ```rust
//! use task_core::workspace_setup::{WorkspaceSetupService, AiToolType, PrdDocument};
//!
//! let prd = PrdDocument::from_file("./docs/PRD.md").await?;
//! let service = WorkspaceSetupService::new();
//! 
//! let instructions = service.get_setup_instructions(AiToolType::ClaudeCode).await?;
//! let workflow = service.analyze_prd_for_agentic_workflow(&prd).await?;
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Supported AI tool types for workspace generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiToolType {
    /// Claude Code by Anthropic
    ClaudeCode,
    /// Future: AutoGen framework  
    AutoGen,
    /// Future: CrewAI framework
    CrewAi,
}

impl std::fmt::Display for AiToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiToolType::ClaudeCode => write!(f, "claude-code"),
            AiToolType::AutoGen => write!(f, "autogen"),
            AiToolType::CrewAi => write!(f, "crewai"),
        }
    }
}

/// Parsed PRD document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrdDocument {
    /// Project title extracted from PRD
    pub title: String,
    /// Project overview/summary
    pub overview: Option<String>,
    /// User stories or requirements
    pub user_stories: Vec<String>,
    /// Technical requirements
    pub technical_requirements: Vec<String>,
    /// Success criteria
    pub success_criteria: Vec<String>,
    /// Constraints and assumptions
    pub constraints: Vec<String>,
    /// Raw PRD content for LLM analysis
    pub raw_content: String,
    /// Validation errors (if any)
    pub validation_errors: Vec<String>,
}

/// Setup instructions returned by the MCP function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupInstructions {
    /// Schema version for compatibility
    pub schema_version: String,
    /// Target AI tool type
    pub ai_tool_type: AiToolType,
    /// Step-by-step setup process
    pub setup_steps: Vec<SetupStep>,
    /// Required MCP functions to call during setup
    pub required_mcp_functions: Vec<RequiredMcpFunction>,
    /// Template instructions for manifest creation
    pub manifest_template: ManifestTemplate,
}

/// Individual setup step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStep {
    /// Step identifier
    pub id: String,
    /// Human-readable step name
    pub name: String,
    /// Detailed description
    pub description: String,
    /// Order in the setup process
    pub order: u32,
    /// Whether this step is required
    pub required: bool,
    /// Expected duration in seconds
    pub estimated_duration_seconds: Option<u32>,
}

/// MCP function that must be called during setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredMcpFunction {
    /// Function name (e.g., "axon:registerAgent")
    pub function_name: String,
    /// Description of when to call this function
    pub when_to_call: String,
    /// Expected parameter structure (JSON schema)
    pub parameter_schema: serde_json::Value,
    /// Example parameters
    pub example_parameters: serde_json::Value,
}

/// Template for creating workspace manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestTemplate {
    /// Target file path (e.g., ".axon/manifest.json")
    pub target_path: String,
    /// JSON schema for the manifest structure
    pub schema: serde_json::Value,
    /// Example manifest content
    pub example: serde_json::Value,
}

/// Agentic workflow description based on PRD analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgenticWorkflowDescription {
    /// Overall workflow summary
    pub workflow_description: String,
    /// Suggested number of agents
    pub recommended_agent_count: u32,
    /// Suggested agent roles with basic descriptions
    pub suggested_agents: Vec<SuggestedAgent>,
    /// Task decomposition strategy
    pub task_decomposition_strategy: String,
    /// Coordination patterns
    pub coordination_patterns: Vec<String>,
    /// Expected workflow steps
    pub workflow_steps: Vec<WorkflowStep>,
}

/// Suggested agent based on PRD analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAgent {
    /// Agent name (kebab-case)
    pub name: String,
    /// Brief description (max 300 chars)
    pub description: String,
    /// Required capabilities/skills
    pub required_capabilities: Vec<String>,
    /// Estimated workload percentage
    pub workload_percentage: f32,
    /// Dependencies on other agents
    pub depends_on: Vec<String>,
}

/// Workflow step in the agentic process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Step identifier
    pub id: String,
    /// Step name
    pub name: String,
    /// Which agent is responsible
    pub responsible_agent: String,
    /// Input requirements
    pub inputs: Vec<String>,
    /// Expected outputs
    pub outputs: Vec<String>,
    /// Step order
    pub order: u32,
}

/// Agent registration data for MCP function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    /// Agent name (kebab-case)
    pub name: String,
    /// Agent description (max 300 chars)
    pub description: String,
    /// Full agent prompt/instructions
    pub prompt: String,
    /// Required capabilities
    pub capabilities: Vec<String>,
    /// Target AI tool type
    pub ai_tool_type: AiToolType,
    /// Dependencies on other agents
    pub dependencies: Vec<String>,
}

/// Main AI file creation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainAiFileData {
    /// Target AI tool type
    pub ai_tool_type: AiToolType,
    /// File name (e.g., "CLAUDE.md")
    pub file_name: String,
    /// Full file content
    pub content: String,
    /// File sections breakdown
    pub sections: Vec<FileSection>,
}

/// Section within the main AI file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSection {
    /// Section title
    pub title: String,
    /// Section content
    pub content: String,
    /// Section order
    pub order: u32,
}

/// Complete workspace manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    /// Schema version
    pub schema_version: String,
    /// Target AI tool type
    pub ai_tool_type: AiToolType,
    /// Project metadata
    pub project: ProjectMetadata,
    /// Registered agents
    pub agents: Vec<AgentRegistration>,
    /// Agentic workflow description
    pub workflow: AgenticWorkflowDescription,
    /// Setup instructions
    pub setup_instructions: Vec<SetupStep>,
    /// Generated files
    pub generated_files: Vec<GeneratedFile>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Axon version used
    pub axon_version: String,
}

/// Project metadata from PRD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project name/title
    pub name: String,
    /// Project description
    pub description: String,
    /// Estimated complexity (1-10 scale)
    pub complexity_score: u32,
    /// Primary domain (e.g., "web-development", "data-analysis")
    pub primary_domain: String,
    /// Required technologies
    pub technologies: Vec<String>,
}

/// Information about generated files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    /// File path relative to project root
    pub path: String,
    /// File type/purpose
    pub file_type: String,
    /// Human-readable description
    pub description: String,
    /// Whether file is critical for functionality
    pub critical: bool,
}

/// Instructions for creating main AI tool file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainAiFileInstructions {
    /// Target AI tool type
    pub ai_tool_type: AiToolType,
    /// Recommended file name
    pub file_name: String,
    /// File structure template
    pub structure_template: Vec<SectionTemplate>,
    /// Content generation guidelines
    pub content_guidelines: Vec<String>,
    /// Example content snippets
    pub examples: HashMap<String, String>,
}

/// Template for a file section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionTemplate {
    /// Section identifier
    pub id: String,
    /// Section title
    pub title: String,
    /// Content template with placeholders
    pub template: String,
    /// Section order
    pub order: u32,
    /// Whether section is required
    pub required: bool,
    /// Placeholder descriptions
    pub placeholders: HashMap<String, String>,
}

/// Error types specific to workspace setup
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceSetupError {
    #[error("PRD parsing failed: {0}")]
    PrdParsingFailed(String),
    
    #[error("PRD validation failed: {errors:?}")]
    PrdValidationFailed { errors: Vec<String> },
    
    #[error("Unsupported AI tool type: {0}")]
    UnsupportedAiTool(String),
    
    #[error("Agent generation failed: {0}")]
    AgentGenerationFailed(String),
    
    #[error("Template rendering failed: {0}")]
    TemplateRenderingFailed(String),
    
    #[error("File system error: {0}")]
    FileSystemError(String),
    
    #[error("LLM API error: {0}")]
    LlmApiError(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

pub type WorkspaceSetupResult<T> = std::result::Result<T, WorkspaceSetupError>;

/// Service for workspace setup operations
#[derive(Clone)]
pub struct WorkspaceSetupService {
    /// Configuration for the service
    config: WorkspaceSetupConfig,
}

/// Configuration for workspace setup service
#[derive(Debug, Clone)]
pub struct WorkspaceSetupConfig {
    /// Maximum number of agents to generate
    pub max_agents: u32,
    /// Default complexity score for projects
    pub default_complexity_score: u32,
    /// Supported AI tool types
    pub supported_ai_tools: Vec<AiToolType>,
    /// Template directory path
    pub template_dir: Option<String>,
}

impl Default for WorkspaceSetupConfig {
    fn default() -> Self {
        Self {
            max_agents: 10,
            default_complexity_score: 5,
            supported_ai_tools: vec![AiToolType::ClaudeCode],
            template_dir: None,
        }
    }
}

impl WorkspaceSetupService {
    /// Create new workspace setup service
    pub fn new() -> Self {
        Self {
            config: WorkspaceSetupConfig::default(),
        }
    }
    
    /// Create service with custom configuration
    pub fn with_config(config: WorkspaceSetupConfig) -> Self {
        Self { config }
    }
    
    /// Get setup instructions for specified AI tool type
    pub async fn get_setup_instructions(&self, ai_tool_type: AiToolType) -> WorkspaceSetupResult<SetupInstructions> {
        if !self.config.supported_ai_tools.contains(&ai_tool_type) {
            return Err(WorkspaceSetupError::UnsupportedAiTool(ai_tool_type.to_string()));
        }
        
        // Generate setup instructions based on AI tool type
        let instructions = match ai_tool_type {
            AiToolType::ClaudeCode => self.generate_claude_code_instructions().await?,
            AiToolType::AutoGen => todo!("AutoGen support not yet implemented"),
            AiToolType::CrewAi => todo!("CrewAI support not yet implemented"),
        };
        
        Ok(instructions)
    }
    
    /// Analyze PRD document for agentic workflow recommendations
    pub async fn get_agentic_workflow_description(&self, prd: &PrdDocument) -> WorkspaceSetupResult<AgenticWorkflowDescription> {
        // This would typically call an LLM API to analyze the PRD
        // For now, we'll provide a basic implementation
        
        let complexity_factor = self.estimate_complexity(&prd)?;
        let recommended_agent_count = std::cmp::min(
            (complexity_factor as f32 * 1.5).ceil() as u32,
            self.config.max_agents
        );
        
        // Generate suggested agents based on PRD content
        let suggested_agents = self.generate_suggested_agents(prd, recommended_agent_count).await?;
        
        Ok(AgenticWorkflowDescription {
            workflow_description: format!(
                "Based on the PRD analysis, this project requires {} agents working in coordination. The workflow follows a hierarchical pattern where a project manager coordinates with specialized agents.",
                recommended_agent_count
            ),
            recommended_agent_count,
            suggested_agents,
            task_decomposition_strategy: "Hierarchical decomposition with capability-based assignment".to_string(),
            coordination_patterns: vec![
                "Project Manager → Specialized Agents".to_string(),
                "Sequential handoffs with validation".to_string(),
                "Parallel execution where possible".to_string(),
            ],
            workflow_steps: vec![], // Would be populated based on PRD analysis
        })
    }
    
    /// Get instructions for creating main AI file (CLAUDE.md, etc.)
    pub async fn get_instructions_for_main_ai_file(&self, ai_tool_type: AiToolType) -> WorkspaceSetupResult<MainAiFileInstructions> {
        let instructions = match ai_tool_type {
            AiToolType::ClaudeCode => self.generate_claude_md_instructions().await?,
            AiToolType::AutoGen => todo!("AutoGen support not yet implemented"),
            AiToolType::CrewAi => todo!("CrewAI support not yet implemented"),
        };
        
        Ok(instructions)
    }
    
    // Private helper methods
    
    async fn generate_claude_code_instructions(&self) -> WorkspaceSetupResult<SetupInstructions> {
        Ok(SetupInstructions {
            schema_version: "1.0".to_string(),
            ai_tool_type: AiToolType::ClaudeCode,
            setup_steps: vec![
                SetupStep {
                    id: "parse-prd".to_string(),
                    name: "Parse PRD Document".to_string(),
                    description: "Extract project requirements and analyze complexity".to_string(),
                    order: 1,
                    required: true,
                    estimated_duration_seconds: Some(30),
                },
                SetupStep {
                    id: "analyze-workflow".to_string(),
                    name: "Analyze Agentic Workflow".to_string(),
                    description: "Determine optimal agent roles and coordination patterns".to_string(),
                    order: 2,
                    required: true,
                    estimated_duration_seconds: Some(45),
                },
                SetupStep {
                    id: "register-agents".to_string(),
                    name: "Register AI Agents".to_string(),
                    description: "Create and register all required AI agent definitions".to_string(),
                    order: 3,
                    required: true,
                    estimated_duration_seconds: Some(60),
                },
                SetupStep {
                    id: "create-main-file".to_string(),
                    name: "Create CLAUDE.md".to_string(),
                    description: "Generate main coordination file for Claude Code".to_string(),
                    order: 4,
                    required: true,
                    estimated_duration_seconds: Some(30),
                },
                SetupStep {
                    id: "create-manifest".to_string(),
                    name: "Create Workspace Manifest".to_string(),
                    description: "Generate .axon/manifest.json with complete setup metadata".to_string(),
                    order: 5,
                    required: true,
                    estimated_duration_seconds: Some(15),
                },
            ],
            required_mcp_functions: vec![
                RequiredMcpFunction {
                    function_name: "axon:getAgenticWorkflowDescription".to_string(),
                    when_to_call: "After parsing PRD to get workflow recommendations".to_string(),
                    parameter_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "prd_content": {"type": "string"},
                            "requested_agent_count": {"type": "integer", "minimum": 1, "maximum": 10}
                        },
                        "required": ["prd_content"]
                    }),
                    example_parameters: serde_json::json!({
                        "prd_content": "# Project: E-commerce Platform\\n\\n## Overview\\n...",
                        "requested_agent_count": 5
                    }),
                },
                RequiredMcpFunction {
                    function_name: "axon:registerAgent".to_string(),
                    when_to_call: "For each agent recommended by workflow analysis".to_string(),
                    parameter_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "name": {"type": "string", "pattern": "^[a-z][a-z0-9-]*$"},
                            "description": {"type": "string", "maxLength": 300},
                            "prompt": {"type": "string"},
                            "capabilities": {"type": "array", "items": {"type": "string"}},
                            "ai_tool_type": {"type": "string", "enum": ["claude-code"]}
                        },
                        "required": ["name", "description", "prompt", "ai_tool_type"]
                    }),
                    example_parameters: serde_json::json!({
                        "name": "project-manager",
                        "description": "Coordinates overall project execution and manages task assignments between specialized agents",
                        "prompt": "You are a project manager AI agent responsible for...",
                        "capabilities": ["project-management", "task-decomposition", "coordination"],
                        "ai_tool_type": "claude-code"
                    }),
                },
            ],
            manifest_template: ManifestTemplate {
                target_path: ".axon/manifest.json".to_string(),
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "schema_version": {"type": "string"},
                        "ai_tool_type": {"type": "string"},
                        "project": {"type": "object"},
                        "agents": {"type": "array"},
                        "workflow": {"type": "object"}
                    }
                }),
                example: serde_json::json!({
                    "schema_version": "1.0",
                    "ai_tool_type": "claude-code",
                    "project": {
                        "name": "E-commerce Platform",
                        "description": "Modern e-commerce platform with AI recommendations"
                    }
                }),
            },
        })
    }
    
    async fn generate_claude_md_instructions(&self) -> WorkspaceSetupResult<MainAiFileInstructions> {
        Ok(MainAiFileInstructions {
            ai_tool_type: AiToolType::ClaudeCode,
            file_name: "CLAUDE.md".to_string(),
            structure_template: vec![
                SectionTemplate {
                    id: "project-overview".to_string(),
                    title: "Project Overview".to_string(),
                    template: "# {{project_name}}\n\n{{project_description}}\n\n## Objectives\n{{objectives}}".to_string(),
                    order: 1,
                    required: true,
                    placeholders: {
                        let mut map = HashMap::new();
                        map.insert("project_name".to_string(), "Name of the project from PRD".to_string());
                        map.insert("project_description".to_string(), "Brief project description".to_string());
                        map.insert("objectives".to_string(), "Main project objectives".to_string());
                        map
                    },
                },
                SectionTemplate {
                    id: "agent-coordination".to_string(),
                    title: "Agent Coordination".to_string(),
                    template: "## Agent Roles\n\n{{agent_descriptions}}\n\n## Workflow\n{{workflow_description}}".to_string(),
                    order: 2,
                    required: true,
                    placeholders: {
                        let mut map = HashMap::new();
                        map.insert("agent_descriptions".to_string(), "Descriptions of all registered agents".to_string());
                        map.insert("workflow_description".to_string(), "How agents coordinate and handoff work".to_string());
                        map
                    },
                },
            ],
            content_guidelines: vec![
                "Use clear, actionable language for AI agents".to_string(),
                "Include specific examples of expected inputs/outputs".to_string(),
                "Provide escalation procedures for edge cases".to_string(),
                "Reference Axon MCP functions for task coordination".to_string(),
            ],
            examples: {
                let mut map = HashMap::new();
                map.insert("agent_prompt_template".to_string(), 
                    "You are a {{role}} agent responsible for {{responsibilities}}. Use Axon MCP functions to coordinate with other agents.".to_string());
                map
            },
        })
    }
    
    fn estimate_complexity(&self, prd: &PrdDocument) -> WorkspaceSetupResult<u32> {
        // Simple heuristic based on PRD content
        let mut complexity = self.config.default_complexity_score;
        
        // Factor in number of requirements
        complexity += (prd.user_stories.len() / 3) as u32;
        complexity += (prd.technical_requirements.len() / 2) as u32;
        
        // Factor in content length (rough proxy for project size)
        let content_length_factor = (prd.raw_content.len() / 1000) as u32;
        complexity += content_length_factor.min(5);
        
        Ok(complexity.min(10))
    }
    
    async fn generate_suggested_agents(&self, prd: &PrdDocument, count: u32) -> WorkspaceSetupResult<Vec<SuggestedAgent>> {
        // This would typically use LLM API to analyze PRD and suggest agents
        // For now, providing a basic implementation based on common patterns
        
        let mut agents = Vec::new();
        
        // Always include a project manager for coordination
        agents.push(SuggestedAgent {
            name: "project-manager".to_string(),
            description: "Coordinates overall project execution and manages task assignments".to_string(),
            required_capabilities: vec!["project-management".to_string(), "coordination".to_string()],
            workload_percentage: 100.0 / count as f32,
            depends_on: vec![],
        });
        
        // Add domain-specific agents based on PRD content analysis
        if prd.raw_content.to_lowercase().contains("api") || prd.raw_content.to_lowercase().contains("backend") {
            agents.push(SuggestedAgent {
                name: "backend-developer".to_string(),
                description: "Implements server-side logic, APIs, and database integration".to_string(),
                required_capabilities: vec!["backend-development".to_string(), "api-design".to_string()],
                workload_percentage: 100.0 / count as f32,
                depends_on: vec!["project-manager".to_string()],
            });
        }
        
        if prd.raw_content.to_lowercase().contains("ui") || prd.raw_content.to_lowercase().contains("frontend") {
            agents.push(SuggestedAgent {
                name: "frontend-developer".to_string(),
                description: "Creates user interfaces and client-side application logic".to_string(),
                required_capabilities: vec!["frontend-development".to_string(), "ui-design".to_string()],
                workload_percentage: 100.0 / count as f32,
                depends_on: vec!["project-manager".to_string()],
            });
        }
        
        // Add QA agent for larger projects
        if count >= 4 {
            agents.push(SuggestedAgent {
                name: "qa-engineer".to_string(),
                description: "Ensures code quality through testing and review processes".to_string(),
                required_capabilities: vec!["testing".to_string(), "quality-assurance".to_string()],
                workload_percentage: 100.0 / count as f32,
                depends_on: vec!["backend-developer".to_string(), "frontend-developer".to_string()],
            });
        }
        
        // Truncate to requested count
        agents.truncate(count as usize);
        
        Ok(agents)
    }
}

impl PrdDocument {
    /// Parse PRD document from file content
    pub fn from_content(content: &str) -> WorkspaceSetupResult<Self> {
        let mut validation_errors = Vec::new();
        
        // Extract title (usually first # header)
        let title = content.lines()
            .find(|line| line.starts_with("# "))
            .map(|line| line.trim_start_matches("# ").trim().to_string())
            .unwrap_or_else(|| {
                validation_errors.push("No title found (expected # header)".to_string());
                "Untitled Project".to_string()
            });
        
        // Basic section extraction (would be more sophisticated in real implementation)
        let overview = Self::extract_section(content, &["overview", "summary", "description"]);
        let user_stories = Self::extract_list_items(content, &["user stories", "requirements", "features"]);
        let technical_requirements = Self::extract_list_items(content, &["technical", "technology", "tech stack"]);
        let success_criteria = Self::extract_list_items(content, &["success", "criteria", "goals"]);
        let constraints = Self::extract_list_items(content, &["constraints", "limitations", "assumptions"]);
        
        // Basic validation
        if content.len() < 100 {
            validation_errors.push("PRD content too short (minimum 100 characters)".to_string());
        }
        
        if user_stories.is_empty() && technical_requirements.is_empty() {
            validation_errors.push("No requirements or user stories found".to_string());
        }
        
        Ok(Self {
            title,
            overview,
            user_stories,
            technical_requirements,
            success_criteria,
            constraints,
            raw_content: content.to_string(),
            validation_errors,
        })
    }
    
    /// Check if PRD is valid for workspace setup
    pub fn is_valid(&self) -> bool {
        self.validation_errors.is_empty()
    }
    
    /// Get validation errors
    pub fn get_validation_errors(&self) -> &[String] {
        &self.validation_errors
    }
    
    // Helper methods for content extraction
    fn extract_section(content: &str, section_headers: &[&str]) -> Option<String> {
        for header in section_headers {
            if let Some(section_content) = Self::find_section_content(content, header) {
                return Some(section_content);
            }
        }
        None
    }
    
    fn extract_list_items(content: &str, section_headers: &[&str]) -> Vec<String> {
        for header in section_headers {
            if let Some(section_content) = Self::find_section_content(content, header) {
                return Self::parse_list_items(&section_content);
            }
        }
        Vec::new()
    }
    
    fn find_section_content(content: &str, header: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        
        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains(&header.to_lowercase()) && 
               (line.starts_with("#") || line.starts_with("##")) {
                // Found the header, now extract content until next header
                let mut section_content = String::new();
                for j in (i + 1)..lines.len() {
                    if lines[j].starts_with("#") {
                        break; // Next section
                    }
                    section_content.push_str(lines[j]);
                    section_content.push('\n');
                }
                return Some(section_content.trim().to_string());
            }
        }
        None
    }
    
    fn parse_list_items(content: &str) -> Vec<String> {
        content.lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    Some(trimmed.trim_start_matches("- ").trim_start_matches("* ").to_string())
                } else if trimmed.chars().next().map_or(false, |c| c.is_ascii_digit()) 
                    && trimmed.contains(". ") {
                    // Numbered list item
                    if let Some(pos) = trimmed.find(". ") {
                        Some(trimmed[pos + 2..].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_tool_type_display() {
        assert_eq!(AiToolType::ClaudeCode.to_string(), "claude-code");
        assert_eq!(AiToolType::AutoGen.to_string(), "autogen");
        assert_eq!(AiToolType::CrewAi.to_string(), "crewai");
    }

    #[test]
    fn test_prd_document_parsing() {
        let content = r#"
# Test Project

## Overview
This is a test project for workspace setup.

## User Stories
- As a user, I want to create an account
- As a user, I want to login securely

## Technical Requirements
- REST API with authentication
- React frontend
- PostgreSQL database
"#;

        let prd = PrdDocument::from_content(content).unwrap();
        assert_eq!(prd.title, "Test Project");
        assert!(prd.overview.is_some());
        assert_eq!(prd.user_stories.len(), 2);
        assert_eq!(prd.technical_requirements.len(), 3);
        assert!(prd.is_valid());
    }

    #[tokio::test]
    async fn test_workspace_setup_service() {
        let service = WorkspaceSetupService::new();
        
        let instructions = service.get_setup_instructions(AiToolType::ClaudeCode).await.unwrap();
        assert_eq!(instructions.ai_tool_type, AiToolType::ClaudeCode);
        assert!(!instructions.setup_steps.is_empty());
        assert!(!instructions.required_mcp_functions.is_empty());
    }

    #[tokio::test]
    async fn test_main_ai_file_instructions() {
        let service = WorkspaceSetupService::new();
        
        let instructions = service.get_instructions_for_main_ai_file(AiToolType::ClaudeCode).await.unwrap();
        assert_eq!(instructions.file_name, "CLAUDE.md");
        assert!(!instructions.structure_template.is_empty());
        assert!(!instructions.content_guidelines.is_empty());
    }
}