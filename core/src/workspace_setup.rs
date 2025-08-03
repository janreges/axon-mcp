//! Workspace Setup Automation for Axon MCP
//!
//! This module provides comprehensive workspace setup automation capabilities
//! that enable AI tools like Claude Code to automatically analyze PRD documents
//! and generate complete AI workspace configurations through MCP function calls.
//!
//! ## 🎯 Key Features
//!
//! - **PRD Analysis**: Intelligent parsing and validation of Product Requirements Documents
//! - **Agent Generation**: AI-powered recommendation of optimal agent roles and capabilities  
//! - **Interactive Workflow**: "Propose → Confirm → Execute" pattern with user control
//! - **Structured Responses**: Consistent JSON responses with status, message, and next steps
//! - **Extensible Design**: Template-based system for supporting different AI tools
//!
//! ## 🔄 MCP Function Flow
//!
//! ```text
//! 1. get_setup_instructions(ai_tool_type) → Setup process overview
//! 2. get_agentic_workflow_description(prd_content) → Agent recommendations  
//! 3. register_agent(agent_data) → Store agent configurations
//! 4. get_main_file_instructions(ai_tool_type) → CLAUDE.md template
//! 5. create_main_file(content, ai_tool_type) → Generate coordination file
//! 6. generate_workspace_manifest(metadata) → Create .axon/manifest.json
//! ```

use crate::prompt_templates::EnhancedPromptBuilder;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported AI tool types for workspace generation
///
/// Currently only Claude Code is supported, with plans for AutoGen and CrewAI in the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiToolType {
    /// Claude Code by Anthropic - currently the only supported tool
    #[serde(rename = "claude-code")]
    ClaudeCode,
    /// AutoGen framework by Microsoft (placeholder)
    #[serde(rename = "autogen")]
    AutoGen,
    /// CrewAI framework (placeholder)
    #[serde(rename = "crew-ai")]
    CrewAi,
}

/// Project archetype classification for better agent generation
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectArchetype {
    CliTool,        // Command-line utilities and tools
    WebApplication, // Full-stack web applications
    DataProcessing, // ETL pipelines, data analysis
    Library,        // SDKs, libraries, frameworks
    MobileApp,      // iOS/Android applications
    Script,         // Automation scripts, one-off tasks
    DesktopApp,     // Desktop GUI applications
    ApiService,     // Pure API/microservice
    Generic,        // Fallback for unclassifiable projects
}

impl std::fmt::Display for AiToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiToolType::ClaudeCode => write!(f, "claude-code"),
            AiToolType::AutoGen => write!(f, "autogen"),
            AiToolType::CrewAi => write!(f, "crew-ai"),
        }
    }
}

impl std::fmt::Display for ProjectArchetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectArchetype::CliTool => write!(f, "CLI Tool"),
            ProjectArchetype::WebApplication => write!(f, "Web Application"),
            ProjectArchetype::DataProcessing => write!(f, "Data Processing"),
            ProjectArchetype::Library => write!(f, "Library"),
            ProjectArchetype::MobileApp => write!(f, "Mobile App"),
            ProjectArchetype::Script => write!(f, "Script"),
            ProjectArchetype::DesktopApp => write!(f, "Desktop App"),
            ProjectArchetype::ApiService => write!(f, "API Service"),
            ProjectArchetype::Generic => write!(f, "Generic Project"),
        }
    }
}

/// Response status for workspace setup MCP functions
///
/// Follows the "Propose → Confirm → Execute" pattern recommended by experts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// Operation completed successfully
    #[serde(rename = "success")]
    Success,
    /// User confirmation required before proceeding
    #[serde(rename = "confirmation_required")]
    ConfirmationRequired,
    /// Error occurred with suggested solutions
    #[serde(rename = "error")]
    Error,
    /// Operation in progress (for long-running tasks)
    #[serde(rename = "in_progress")]
    InProgress,
}

/// Next action that user can take
///
/// Provides structured options for continuing the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextStep {
    /// User-friendly label for the action
    pub label: String,
    /// Action identifier for programmatic use
    pub action: String,
    /// Whether this is the recommended default action
    pub is_default: bool,
}

/// Standard response wrapper for all workspace setup MCP functions
///
/// Provides consistent structure with user feedback, next steps, and debugging info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSetupResponse<T> {
    /// Status of the operation
    pub status: ResponseStatus,
    /// Human-readable message for the user
    pub message: String,
    /// Function-specific payload data
    pub payload: T,
    /// Available next actions for the user
    pub next_steps: Vec<NextStep>,
    /// Technical logs for debugging (hidden from user)
    pub logs: Vec<String>,
}

impl<T> WorkspaceSetupResponse<T> {
    /// Create a successful response
    pub fn success(message: String, payload: T) -> Self {
        Self {
            status: ResponseStatus::Success,
            message,
            payload,
            next_steps: vec![],
            logs: vec![],
        }
    }

    /// Create a response requiring user confirmation
    pub fn confirmation_required(message: String, payload: T, next_steps: Vec<NextStep>) -> Self {
        Self {
            status: ResponseStatus::ConfirmationRequired,
            message,
            payload,
            next_steps,
            logs: vec![],
        }
    }

    /// Create an error response with suggested solutions
    pub fn error(message: String, payload: T) -> Self {
        Self {
            status: ResponseStatus::Error,
            message,
            payload,
            next_steps: vec![],
            logs: vec![],
        }
    }

    /// Add a log entry for debugging
    pub fn with_log(mut self, log: String) -> Self {
        self.logs.push(log);
        self
    }

    /// Add multiple log entries
    pub fn with_logs(mut self, logs: Vec<String>) -> Self {
        self.logs.extend(logs);
        self
    }
}

/// Parsed PRD (Product Requirements Document) structure
///
/// Represents a structured view of a PRD with validation and complexity analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrdDocument {
    /// Project title extracted from PRD
    pub title: String,
    /// Project overview/summary section
    pub overview: Option<String>,
    /// Project objectives and goals
    pub objectives: Vec<String>,
    /// User stories, features, or functional requirements
    pub user_stories: Vec<String>,
    /// Technical requirements and technology stack
    pub technical_requirements: Vec<String>,
    /// Success criteria and acceptance criteria
    pub success_criteria: Vec<String>,
    /// Constraints, limitations, and assumptions
    pub constraints: Vec<String>,
    /// Timeline or schedule information
    pub timeline: Option<String>,
    /// Raw PRD content for reference
    pub raw_content: String,
    /// Validation errors found during parsing
    pub validation_errors: Vec<String>,
}

impl PrdDocument {
    /// Parse PRD content from markdown text
    ///
    /// Performs intelligent extraction of common PRD sections using header matching
    pub fn from_content(content: &str) -> Result<Self, WorkspaceSetupError> {
        let mut validation_errors = Vec::new();

        // Basic content validation
        if content.trim().len() < 50 {
            validation_errors.push("PRD content too short. Please provide a comprehensive PRD with objectives, technical requirements, and user stories.".to_string());
        }

        // Extract title (usually first # header)
        let title = content
            .lines()
            .find(|line| line.trim().starts_with("# "))
            .map(|line| line.trim_start_matches("# ").trim().to_string())
            .unwrap_or_else(|| {
                validation_errors.push(
                    "No project title found. Please add a title using '# Project Name' format."
                        .to_string(),
                );
                "Untitled Project".to_string()
            });

        // Extract sections using intelligent matching
        let overview =
            Self::extract_section(content, &["overview", "summary", "description", "about"]);
        let objectives =
            Self::extract_list_items(content, &["objectives", "goals", "purpose", "aims"]);
        let user_stories = Self::extract_list_items(
            content,
            &["user stories", "requirements", "features", "functionality"],
        );
        let technical_requirements = Self::extract_list_items(
            content,
            &[
                "technical",
                "technology",
                "tech stack",
                "architecture",
                "implementation",
            ],
        );
        let success_criteria = Self::extract_list_items(
            content,
            &["success", "criteria", "acceptance", "definition of done"],
        );
        let constraints =
            Self::extract_list_items(content, &["constraints", "limitations", "assumptions"]);
        let timeline =
            Self::extract_section(content, &["timeline", "schedule", "milestones", "roadmap"]);

        // Validation logic
        if objectives.is_empty() {
            validation_errors.push("No project objectives found. Please add an 'Objectives' or 'Goals' section with bullet points.".to_string());
        }

        if technical_requirements.is_empty() {
            validation_errors.push("No technical requirements found. Please add a 'Technical Requirements' or 'Tech Stack' section.".to_string());
        }

        if user_stories.is_empty() {
            validation_errors.push("No user stories or features found. Please add a 'User Stories' or 'Features' section.".to_string());
        }

        Ok(Self {
            title,
            overview,
            objectives,
            user_stories,
            technical_requirements,
            success_criteria,
            constraints,
            timeline,
            raw_content: content.to_string(),
            validation_errors,
        })
    }

    /// Check if PRD meets minimum requirements for workspace generation
    pub fn is_valid(&self) -> bool {
        self.validation_errors.is_empty()
    }

    /// Get list of validation errors with suggestions
    pub fn get_validation_errors(&self) -> &[String] {
        &self.validation_errors
    }

    /// Calculate project complexity score (1-10 scale)
    pub fn calculate_complexity_score(&self) -> u8 {
        let mut score = 1u8;

        // Base complexity from technical requirements
        score += (self.technical_requirements.len() / 2) as u8;

        // Additional complexity from user stories
        score += (self.user_stories.len() / 3) as u8;

        // Content length factor
        score += (self.raw_content.len() / 2000) as u8;

        // Complexity keywords in technical requirements
        let high_complexity_keywords = [
            "microservices",
            "distributed",
            "real-time",
            "machine learning",
            "ai",
            "ml",
            "blockchain",
            "kubernetes",
            "docker",
            "scaling",
            "performance",
            "security",
            "authentication",
            "authorization",
            "encryption",
            "mobile",
            "cross-platform",
            "api gateway",
            "message queue",
            "event-driven",
            "serverless",
            "cloud-native",
        ];

        for req in &self.technical_requirements {
            let req_lower = req.to_lowercase();

            // High complexity keywords add more points
            for keyword in &high_complexity_keywords {
                if req_lower.contains(keyword) {
                    score += 2;
                    break;
                }
            }
        }

        // Cap at 10
        score.min(10)
    }

    /// Suggest optimal number of agents based on complexity
    pub fn suggest_agent_count(&self) -> u8 {
        match self.calculate_complexity_score() {
            1..=2 => 2,  // Simple projects: PM + 1 specialist
            3..=4 => 3,  // Basic projects: PM + 2 specialists
            5..=6 => 4,  // Medium complexity: PM + 3 specialists
            7..=8 => 6,  // Complex projects: PM + 5 specialists
            9..=10 => 8, // Very complex: PM + 7 specialists
            _ => 3,      // Default fallback
        }
    }

    // Private helper methods for content extraction

    fn extract_section(content: &str, section_headers: &[&str]) -> Option<String> {
        for header in section_headers {
            if let Some(section_content) = Self::find_section_content(content, header) {
                if !section_content.trim().is_empty() {
                    return Some(section_content);
                }
            }
        }
        None
    }

    fn extract_list_items(content: &str, section_headers: &[&str]) -> Vec<String> {
        for header in section_headers {
            if let Some(section_content) = Self::find_section_content(content, header) {
                let items = Self::parse_list_items(&section_content);
                if !items.is_empty() {
                    return items;
                }
            }
        }
        Vec::new()
    }

    fn find_section_content(content: &str, header: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let header_lower = header.to_lowercase();

        for (i, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();

            // Look for markdown headers containing our target header
            if (line.starts_with("##") || line.starts_with("#"))
                && line_lower.contains(&header_lower)
            {
                // Extract content until next header at same or higher level
                let current_header_level = line.chars().take_while(|&c| c == '#').count();
                let mut section_content = String::new();

                for next_line in lines.iter().skip(i + 1) {
                    // Check if we hit another header at same or higher level
                    if next_line.starts_with("#") {
                        let next_header_level = next_line.chars().take_while(|&c| c == '#').count();
                        if next_header_level <= current_header_level {
                            break;
                        }
                    }

                    section_content.push_str(next_line);
                    section_content.push('\n');
                }

                let trimmed = section_content.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        None
    }

    fn parse_list_items(content: &str) -> Vec<String> {
        let mut items = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Bullet points (- or *)
            if let Some(stripped) = trimmed.strip_prefix("- ") {
                items.push(stripped.trim().to_string());
            } else if let Some(stripped) = trimmed.strip_prefix("* ") {
                items.push(stripped.trim().to_string());
            }
            // Numbered lists (1. 2. etc.)
            else if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                if let Some(dot_pos) = trimmed.find(". ") {
                    let content = &trimmed[dot_pos + 2..];
                    if !content.trim().is_empty() {
                        items.push(content.trim().to_string());
                    }
                }
            }
        }

        // Deduplicate and filter out very short items
        items
            .into_iter()
            .filter(|item| item.len() > 3)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }
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

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

pub type WorkspaceSetupResult<T> = std::result::Result<T, WorkspaceSetupError>;

// Data structures for MCP functions

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupInstructions {
    pub schema_version: String,
    pub ai_tool_type: AiToolType,
    pub setup_steps: Vec<SetupStep>,
    pub required_mcp_functions: Vec<RequiredMcpFunction>,
    pub manifest_template: ManifestTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStep {
    pub id: String,
    pub name: String,
    pub description: String,
    pub order: u8,
    pub required: bool,
    pub validation_script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredMcpFunction {
    pub function_name: String,
    pub when_to_call: String,
    pub expected_parameters: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestTemplate {
    pub target_path: String,
    pub schema: serde_json::Value,
    pub example: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgenticWorkflowDescription {
    pub workflow_description: String,
    pub recommended_agent_count: u32,
    pub suggested_agents: Vec<SuggestedAgent>,
    pub task_decomposition_strategy: String,
    pub coordination_patterns: Vec<String>,
    pub workflow_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAgent {
    pub name: String,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub workload_percentage: f32,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub capabilities: Vec<String>,
    pub ai_tool_type: AiToolType,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainAiFileInstructions {
    pub ai_tool_type: AiToolType,
    pub file_name: String,
    pub structure_template: Vec<SectionTemplate>,
    pub content_guidelines: Vec<String>,
    pub examples: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionTemplate {
    pub id: String,
    pub title: String,
    pub template: String,
    pub order: u8,
    pub required: bool,
    pub placeholders: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainAiFileData {
    pub ai_tool_type: AiToolType,
    pub file_name: String,
    pub content: String,
    pub sections: Vec<FileSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSection {
    pub title: String,
    pub content: String,
    pub order: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub schema_version: String,
    pub ai_tool_type: AiToolType,
    pub project: ProjectMetadata,
    pub agents: Vec<AgentRegistration>,
    pub workflow: AgenticWorkflowDescription,
    pub setup_instructions: Vec<SetupStep>,
    pub generated_files: Vec<GeneratedFile>,
    pub created_at: DateTime<Utc>,
    pub axon_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub description: String,
    pub complexity_score: u8,
    pub primary_domain: String,
    pub technologies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: String,
    pub file_type: String,
    pub description: String,
    pub critical: bool,
}

/// MCP function parameters

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSetupInstructionsParams {
    pub workspace_id: String,
    pub ai_tool_type: AiToolType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAgenticWorkflowDescriptionParams {
    pub workspace_id: String,
    pub prd_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterAgentParams {
    pub workspace_id: String,
    pub agent: AgentRegistration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMainFileInstructionsParams {
    pub workspace_id: String,
    pub ai_tool_type: AiToolType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMainFileParams {
    pub workspace_id: String,
    pub content: String,
    pub ai_tool_type: AiToolType,
    pub project_name: Option<String>,
    pub overwrite_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateWorkspaceManifestParams {
    pub workspace_id: String,
    pub prd_content: String,
    pub agents: Vec<AgentRegistration>,
    pub include_generated_files: bool,
}

/// Type aliases for cleaner MCP function signatures
pub type SetupInstructionsResponse = WorkspaceSetupResponse<SetupInstructions>;
pub type AgenticWorkflowResponse = WorkspaceSetupResponse<AgenticWorkflowDescription>;
pub type AgentRegistrationResponse = WorkspaceSetupResponse<AgentRegistration>;
pub type MainFileInstructionsResponse = WorkspaceSetupResponse<MainAiFileInstructions>;
pub type MainFileDataResponse = WorkspaceSetupResponse<MainAiFileData>;
pub type WorkspaceManifestResponse = WorkspaceSetupResponse<WorkspaceManifest>;

/// Configuration for workspace setup service
#[derive(Debug, Clone)]
pub struct WorkspaceSetupConfig {
    pub max_agents: u8,
    pub default_agent_count: u8,
    pub supported_ai_tools: Vec<AiToolType>,
    pub template_base_path: String,
}

impl Default for WorkspaceSetupConfig {
    fn default() -> Self {
        Self {
            max_agents: 10,
            default_agent_count: 3,
            supported_ai_tools: vec![AiToolType::ClaudeCode],
            template_base_path: ".axon/templates".to_string(),
        }
    }
}

/// Service for workspace setup operations
///
/// Provides all 6 MCP functions for complete workspace automation
#[derive(Clone)]
pub struct WorkspaceSetupService {
    config: WorkspaceSetupConfig,
    prompt_builder: EnhancedPromptBuilder,
}

impl Default for WorkspaceSetupService {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSetupService {
    /// Create new workspace setup service with default configuration
    pub fn new() -> Self {
        Self {
            config: WorkspaceSetupConfig::default(),
            prompt_builder: EnhancedPromptBuilder::new(),
        }
    }

    /// Create service with custom configuration
    pub fn with_config(config: WorkspaceSetupConfig) -> Self {
        Self {
            config,
            prompt_builder: EnhancedPromptBuilder::new(),
        }
    }

    /// 1️⃣ GET SETUP INSTRUCTIONS
    pub async fn get_setup_instructions(
        &self,
        ai_tool_type: AiToolType,
    ) -> WorkspaceSetupResult<SetupInstructionsResponse> {
        let instructions = SetupInstructions {
            schema_version: "1.0".to_string(),
            ai_tool_type,
            setup_steps: vec![
                SetupStep {
                    id: "analyze-prd".to_string(),
                    name: "Analyze PRD Document".to_string(),
                    description: "Read and analyze the PRD.md file to understand project requirements".to_string(),
                    order: 1,
                    required: true,
                    validation_script: Some("test -f docs/PRD.md".to_string()),
                },
                SetupStep {
                    id: "generate-workflow".to_string(),
                    name: "Generate Agentic Workflow".to_string(),
                    description: "Call get_agentic_workflow_description to analyze PRD and get agent recommendations".to_string(),
                    order: 2,
                    required: true,
                    validation_script: None,
                },
                SetupStep {
                    id: "register-agents".to_string(),
                    name: "Register AI Agents".to_string(),
                    description: "Register each recommended agent using the register_agent MCP function".to_string(),
                    order: 3,
                    required: true,
                    validation_script: None,
                },
            ],
            required_mcp_functions: vec![
                RequiredMcpFunction {
                    function_name: "get_agentic_workflow_description".to_string(),
                    when_to_call: "After reading PRD.md to analyze project and get agent recommendations".to_string(),
                    expected_parameters: "prd_content: full text content of PRD.md file".to_string(),
                },
                RequiredMcpFunction {
                    function_name: "register_agent".to_string(),
                    when_to_call: "For each agent recommended by workflow analysis".to_string(),
                    expected_parameters: "agent: AgentRegistration object with name, description, prompt, capabilities".to_string(),
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
                        "agents": {"type": "array"}
                    }
                }),
                example: serde_json::json!({
                    "schema_version": "1.0",
                    "ai_tool_type": "claude-code",
                    "project": {
                        "name": "Example Project",
                        "description": "Example description"
                    }
                }),
            },
        };

        Ok(WorkspaceSetupResponse::success(
            "Setup instructions generated successfully. Follow these steps to configure your workspace using Axon MCP functions.".to_string(),
            instructions
        ))
    }

    /// 2️⃣ GET AGENTIC WORKFLOW DESCRIPTION
    pub async fn get_agentic_workflow_description(
        &self,
        prd: &PrdDocument,
    ) -> WorkspaceSetupResult<AgenticWorkflowResponse> {
        // Validate PRD first
        if !prd.is_valid() {
            let errors = prd.get_validation_errors();
            let error_msg = format!(
                "PRD validation failed. Please fix these issues before proceeding:\n{}",
                errors
                    .iter()
                    .enumerate()
                    .map(|(i, e)| format!("{}. {}", i + 1, e))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            return Ok(WorkspaceSetupResponse::error(
                error_msg,
                AgenticWorkflowDescription {
                    workflow_description: "Cannot generate workflow from invalid PRD".to_string(),
                    recommended_agent_count: 0,
                    suggested_agents: vec![],
                    task_decomposition_strategy: "N/A".to_string(),
                    coordination_patterns: vec![],
                    workflow_steps: vec![],
                },
            ));
        }

        // PHASE 1: Classify project archetype first
        let archetype = self.classify_project_archetype(prd);

        // PHASE 2: Apply archetype-specific complexity rules
        let (recommended_agent_count, coordination_patterns, task_decomposition_strategy) =
            self.get_archetype_specific_workflow(&archetype, prd);

        // Ensure we respect max agents limit
        let recommended_agent_count = recommended_agent_count.min(self.config.max_agents as u32);

        // Generate suggested agents with enhanced prompts based on archetype
        let suggested_agents = self
            .generate_suggested_agents_for_archetype(&archetype, prd, recommended_agent_count)
            .await?;

        // Generate workflow steps based on project archetype and agents
        let workflow_steps = self.generate_workflow_steps(&archetype, &suggested_agents);

        let workflow = AgenticWorkflowDescription {
            workflow_description: format!(
                "Classified as {} project with complexity score {}/10. Recommending {} agents using {}-specific workflow patterns.",
                archetype,
                prd.calculate_complexity_score(),
                recommended_agent_count,
                archetype
            ),
            recommended_agent_count,
            suggested_agents: suggested_agents.clone(),
            task_decomposition_strategy,
            coordination_patterns,
            workflow_steps,
        };

        let next_steps = vec![
            NextStep {
                label: "Register these agents".to_string(),
                action: "register_agents".to_string(),
                is_default: true,
            },
            NextStep {
                label: format!("Modify agent count (currently {recommended_agent_count})"),
                action: "refine_agent_count".to_string(),
                is_default: false,
            },
        ];

        Ok(WorkspaceSetupResponse::confirmation_required(
            format!(
                "Analyzed PRD '{}' and recommend {} agents for this {} complexity {} project. Review the suggested agents and confirm to proceed.",
                prd.title,
                recommended_agent_count,
                match prd.calculate_complexity_score() {
                    1..=3 => "low",
                    4..=6 => "medium", 
                    7..=8 => "high",
                    _ => "very high",
                },
                archetype
            ),
            workflow,
            next_steps
        ))
    }

    /// 3️⃣ REGISTER AGENT
    pub async fn register_agent(
        &self,
        mut agent: AgentRegistration,
    ) -> WorkspaceSetupResult<AgentRegistrationResponse> {
        // Validate agent data
        if agent.name.trim().is_empty() {
            return Ok(WorkspaceSetupResponse::error(
                "Agent name cannot be empty. Please provide a valid agent name (e.g., 'project-manager').".to_string(),
                agent
            ));
        }

        if agent.description.trim().is_empty() {
            return Ok(WorkspaceSetupResponse::error(
                "Agent description cannot be empty. Please provide a clear description of the agent's role.".to_string(),
                agent
            ));
        }

        if agent.capabilities.is_empty() {
            return Ok(WorkspaceSetupResponse::error(
                "Agent must have at least one capability. Please specify the agent's skills."
                    .to_string(),
                agent,
            ));
        }

        // Generate enhanced prompt using 2025 best practices
        let enhanced_prompt = self.prompt_builder.generate_agent_prompt(
            &SuggestedAgent {
                name: agent.name.clone(),
                description: agent.description.clone(),
                required_capabilities: agent.capabilities.clone(),
                workload_percentage: 100.0, // Default for individual agent registration
                depends_on: agent.dependencies.clone(),
            },
            &ProjectArchetype::Generic, // Default archetype for individual registration
            &format!("Agent registration for: {}", agent.name),
            None,
        );

        // Update agent with enhanced prompt
        agent.prompt = enhanced_prompt;

        let agent_name = agent.name.clone();
        Ok(WorkspaceSetupResponse::success(
            format!(
                "Agent '{}' registered successfully with {} capabilities and enhanced 2025 prompt.",
                agent.name,
                agent.capabilities.len()
            ),
            agent,
        )
        .with_log(format!(
            "Registered agent: {agent_name} with enhanced prompt generation"
        )))
    }

    /// 4️⃣ GET MAIN FILE INSTRUCTIONS
    pub async fn get_main_file_instructions(
        &self,
        ai_tool_type: AiToolType,
    ) -> WorkspaceSetupResult<MainFileInstructionsResponse> {
        let instructions = MainAiFileInstructions {
            ai_tool_type,
            file_name: "CLAUDE.md".to_string(),
            structure_template: vec![SectionTemplate {
                id: "project-header".to_string(),
                title: "Project Header".to_string(),
                template: "# {{project_name}}\n\n{{project_description}}".to_string(),
                order: 1,
                required: true,
                placeholders: {
                    let mut map = HashMap::new();
                    map.insert(
                        "project_name".to_string(),
                        "Name of the project from PRD".to_string(),
                    );
                    map.insert(
                        "project_description".to_string(),
                        "Brief project description".to_string(),
                    );
                    map
                },
            }],
            content_guidelines: vec![
                "Use clear, actionable language for AI agents".to_string(),
                "Include specific examples of MCP function calls".to_string(),
                "Define coordination protocols between agents".to_string(),
            ],
            examples: {
                let mut examples = HashMap::new();
                examples.insert("coordination_example".to_string(),
                    "1. Use list_tasks to find your assigned tasks\n2. Use claim_task to claim available work\n3. Use create_task_message for handoffs".to_string());
                examples
            },
        };

        Ok(WorkspaceSetupResponse::success(
            format!("Main file instructions generated for {ai_tool_type}. Use these templates to create your coordination file."),
            instructions
        ))
    }

    /// 5️⃣ CREATE MAIN FILE
    pub async fn create_main_file(
        &self,
        content: &str,
        ai_tool_type: AiToolType,
        project_name: Option<&str>,
    ) -> WorkspaceSetupResult<MainFileDataResponse> {
        if content.trim().is_empty() {
            return Ok(WorkspaceSetupResponse::error(
                "File content cannot be empty. Please provide the complete content for the main coordination file.".to_string(),
                MainAiFileData {
                    ai_tool_type,
                    file_name: "".to_string(),
                    content: "".to_string(),
                    sections: vec![],
                }
            ));
        }

        let sections = vec![FileSection {
            title: "Project Overview".to_string(),
            content: project_name.unwrap_or("Project").to_string(),
            order: 1,
        }];

        let file_name = match ai_tool_type {
            AiToolType::ClaudeCode => "CLAUDE.md".to_string(),
            AiToolType::AutoGen => "autogen_config.py".to_string(),
            AiToolType::CrewAi => "crew.py".to_string(),
        };

        let file_data = MainAiFileData {
            ai_tool_type,
            file_name: file_name.clone(),
            content: content.to_string(),
            sections,
        };

        Ok(WorkspaceSetupResponse::success(
            format!("Main coordination file '{file_name}' created successfully."),
            file_data,
        ))
    }

    /// 6️⃣ GENERATE WORKSPACE MANIFEST
    pub async fn generate_workspace_manifest(
        &self,
        prd: &PrdDocument,
        agents: &[AgentRegistration],
        include_generated_files: bool,
    ) -> WorkspaceSetupResult<WorkspaceManifestResponse> {
        let workflow_response = self.get_agentic_workflow_description(prd).await?;
        let workflow = workflow_response.payload;

        // Ensure all agents have enhanced prompts
        let enhanced_agents: Vec<AgentRegistration> = agents
            .iter()
            .map(|agent| {
                let enhanced_prompt = self.prompt_builder.generate_agent_prompt(
                    &SuggestedAgent {
                        name: agent.name.clone(),
                        description: agent.description.clone(),
                        required_capabilities: agent.capabilities.clone(),
                        workload_percentage: 100.0 / agents.len() as f32,
                        depends_on: agent.dependencies.clone(),
                    },
                    &self.classify_project_archetype(prd),
                    &format!("Project: {}", prd.title),
                    None,
                );

                AgentRegistration {
                    name: agent.name.clone(),
                    description: agent.description.clone(),
                    prompt: enhanced_prompt,
                    capabilities: agent.capabilities.clone(),
                    ai_tool_type: agent.ai_tool_type,
                    dependencies: agent.dependencies.clone(),
                }
            })
            .collect();

        let manifest = WorkspaceManifest {
            schema_version: "2.0".to_string(), // Updated for enhanced features
            ai_tool_type: AiToolType::ClaudeCode,
            project: ProjectMetadata {
                name: prd.title.clone(),
                description: prd
                    .overview
                    .clone()
                    .unwrap_or_else(|| "No description available".to_string()),
                complexity_score: prd.calculate_complexity_score(),
                primary_domain: "software-development".to_string(),
                technologies: prd.technical_requirements.clone(),
            },
            agents: enhanced_agents,
            workflow,
            setup_instructions: vec![],
            generated_files: if include_generated_files {
                vec![GeneratedFile {
                    path: "CLAUDE.md".to_string(),
                    file_type: "coordination".to_string(),
                    description: "Main coordination file for Claude Code with 2025 enhancements"
                        .to_string(),
                    critical: true,
                }]
            } else {
                vec![]
            },
            created_at: Utc::now(),
            axon_version: "2.0.0".to_string(), // Updated for enhanced features
        };

        Ok(WorkspaceSetupResponse::success(
            format!("Enhanced workspace manifest generated for project '{}' with {} agents using 2025 best practices.", 
                manifest.project.name,
                manifest.agents.len()
            ),
            manifest
        ))
    }

    // Private helper methods

    /// Classify project archetype based on PRD content analysis
    ///
    /// Uses a priority-based approach, checking from most specific to most general
    /// to avoid keyword overlap issues. Falls back to Generic for unclassifiable projects.
    fn classify_project_archetype(&self, prd: &PrdDocument) -> ProjectArchetype {
        let content_lower = format!(
            "{} {} {}",
            prd.title.to_lowercase(),
            prd.overview
                .as_ref()
                .unwrap_or(&String::new())
                .to_lowercase(),
            prd.technical_requirements.join(" ").to_lowercase()
        );

        // 1. Mobile/Desktop - very specific patterns
        if (content_lower.contains("mobile") && !content_lower.contains("web"))
            || content_lower.contains("ios")
            || content_lower.contains("android")
            || content_lower.contains("react native")
            || content_lower.contains("flutter")
            || content_lower.contains("swift")
            || content_lower.contains("kotlin")
            || content_lower.contains("xamarin")
            || content_lower.contains("mobile app")
        {
            return ProjectArchetype::MobileApp;
        }

        if content_lower.contains("desktop")
            || content_lower.contains("gui")
            || content_lower.contains("electron")
            || content_lower.contains("wpf")
            || content_lower.contains("qt")
            || content_lower.contains("tkinter")
            || content_lower.contains(".net maui")
            || content_lower.contains("tauri")
        {
            return ProjectArchetype::DesktopApp;
        }

        // 2. Data Processing - highly specific domain
        if content_lower.contains("etl")
            || content_lower.contains("data processing")
            || content_lower.contains("pipeline")
            || content_lower.contains("analytics")
            || content_lower.contains("machine learning")
            || content_lower.contains("spark")
            || content_lower.contains("hadoop")
            || content_lower.contains("kafka")
            || content_lower.contains("airflow")
            || content_lower.contains("big data")
            || content_lower.contains("data warehouse")
        {
            return ProjectArchetype::DataProcessing;
        }

        // 3. API Service - specific because it lacks frontend
        if (content_lower.contains("api") || content_lower.contains("microservice"))
            && !content_lower.contains("frontend")
            && !content_lower.contains("gui")
            && !content_lower.contains("html")
        {
            return ProjectArchetype::ApiService;
        }

        // 4. Library/SDK - specific development patterns
        if content_lower.contains("library")
            || content_lower.contains("sdk")
            || content_lower.contains("framework")
            || content_lower.contains("package")
            || content_lower.contains("module")
            || content_lower.contains("api design")
            || content_lower.contains("semantic versioning")
        {
            return ProjectArchetype::Library;
        }

        // 5. Web Application - requires frontend AND backend signals
        let has_frontend = content_lower.contains("frontend")
            || content_lower.contains("html")
            || content_lower.contains("css")
            || content_lower.contains("javascript")
            || content_lower.contains("react")
            || content_lower.contains("vue")
            || content_lower.contains("angular");
        let has_backend = content_lower.contains("backend")
            || content_lower.contains("server")
            || content_lower.contains("database")
            || content_lower.contains("api endpoint")
            || content_lower.contains("django")
            || content_lower.contains("rails")
            || content_lower.contains("node.js");

        if (has_frontend && has_backend)
            || content_lower.contains("full-stack")
            || content_lower.contains("web application")
        {
            return ProjectArchetype::WebApplication;
        }

        // 6. CLI Tool - often combined with other terms, but prioritize when no GUI/web
        if (content_lower.contains("cli")
            || content_lower.contains("command-line")
            || content_lower.contains("converter")
            || content_lower.contains("tool"))
            && !content_lower.contains("frontend")
            && !content_lower.contains("gui")
        {
            return ProjectArchetype::CliTool;
        }

        // 7. Script - simple automation scripts
        if content_lower.contains("automation")
            || content_lower.contains("batch")
            || (content_lower.contains("script") && !content_lower.contains("javascript"))
        {
            return ProjectArchetype::Script;
        }

        // 8. FALLBACK: Generic for unclassifiable projects
        eprintln!("⚠️  ARCHETYPE CLASSIFICATION: Project '{}' could not be classified into a specific archetype", prd.title);
        eprintln!(
            "   Content analyzed: {}",
            content_lower.chars().take(200).collect::<String>()
        );
        eprintln!("   Using Generic archetype with default complexity");
        ProjectArchetype::Generic
    }

    /// Apply archetype-specific workflow patterns and complexity rules
    fn get_archetype_specific_workflow(
        &self,
        archetype: &ProjectArchetype,
        prd: &PrdDocument,
    ) -> (u32, Vec<String>, String) {
        match archetype {
            ProjectArchetype::CliTool => {
                // CLI tools are inherently simple - maximum 3 agents
                let agent_count = prd.calculate_complexity_score().clamp(1, 3) as u32;
                (
                    agent_count,
                    vec![
                        "Linear workflow with sequential development".to_string(),
                        "Single developer handles most tasks".to_string(),
                        "PM coordinates and validates".to_string(),
                    ],
                    "Sequential development with minimal coordination overhead".to_string(),
                )
            }
            ProjectArchetype::Script => {
                // Scripts are very simple - usually 1-2 agents
                (
                    2,
                    vec![
                        "Single developer workflow".to_string(),
                        "Optional reviewer for quality assurance".to_string(),
                    ],
                    "Single-agent development with optional review".to_string(),
                )
            }
            ProjectArchetype::Library => {
                // Libraries need more careful design - 3-4 agents
                let agent_count = (prd.calculate_complexity_score() + 1).clamp(2, 4) as u32;
                (
                    agent_count,
                    vec![
                        "API Design → Implementation → Testing".to_string(),
                        "Architecture review before implementation".to_string(),
                        "Documentation-driven development".to_string(),
                    ],
                    "Design-first approach with architectural validation".to_string(),
                )
            }
            ProjectArchetype::ApiService => {
                // APIs need backend focus - 3-5 agents
                let agent_count = (prd.calculate_complexity_score() + 1).clamp(3, 5) as u32;
                (
                    agent_count,
                    vec![
                        "API Design → Backend Implementation → Testing".to_string(),
                        "Database design coordination".to_string(),
                        "Performance and scalability focus".to_string(),
                    ],
                    "Backend-focused with API-first design".to_string(),
                )
            }
            ProjectArchetype::WebApplication => {
                // Web apps use the original complex logic
                let agent_count = prd.suggest_agent_count() as u32;
                (
                    agent_count,
                    vec![
                        "Frontend ↔ Backend coordination".to_string(),
                        "Database design coordination".to_string(),
                        "DevOps and deployment coordination".to_string(),
                        "Testing across all layers".to_string(),
                    ],
                    "Full-stack coordination with parallel development".to_string(),
                )
            }
            ProjectArchetype::MobileApp => {
                // Mobile apps need UI focus - 4-6 agents
                let agent_count = (prd.calculate_complexity_score() + 2).clamp(3, 6) as u32;
                (
                    agent_count,
                    vec![
                        "UI/UX Design → Platform Development → Testing".to_string(),
                        "Cross-platform coordination".to_string(),
                        "App store deployment".to_string(),
                    ],
                    "Mobile-first development with platform-specific optimization".to_string(),
                )
            }
            ProjectArchetype::DesktopApp => {
                // Desktop apps - 3-5 agents
                let agent_count = prd.calculate_complexity_score().clamp(2, 5) as u32;
                (
                    agent_count,
                    vec![
                        "GUI Design → Application Logic → Platform Integration".to_string(),
                        "Cross-platform compatibility testing".to_string(),
                    ],
                    "Desktop application with native OS integration".to_string(),
                )
            }
            ProjectArchetype::DataProcessing => {
                // Data processing - 4-7 agents
                let agent_count = (prd.calculate_complexity_score() + 1).clamp(2, 5) as u32;
                (
                    agent_count,
                    vec![
                        "Data Architecture → Pipeline Implementation → Validation".to_string(),
                        "Performance optimization and monitoring".to_string(),
                    ],
                    "Data-centric pipeline with quality assurance".to_string(),
                )
            }
            ProjectArchetype::Generic => {
                // Log unclassified projects for future classification improvements
                println!("⚠️  UNCLASSIFIED PROJECT: '{}' (complexity: {}) - Consider adding classification rules", 
                    prd.title, prd.calculate_complexity_score());
                println!(
                    "   Technical requirements: {:?}",
                    prd.technical_requirements
                );

                // Bezpečný fallback pro neidentifikovatelné projekty
                let agent_count = prd.calculate_complexity_score().clamp(2, 4) as u32;
                (
                    agent_count,
                    vec![
                        "General project workflow".to_string(),
                        "Adaptive coordination based on requirements".to_string(),
                    ],
                    "Generic project workflow for unclassified archetype".to_string(),
                )
            }
        }
    }

    /// Generate agents specialized for specific project archetype
    async fn generate_suggested_agents_for_archetype(
        &self,
        archetype: &ProjectArchetype,
        prd: &PrdDocument,
        count: u32,
    ) -> WorkspaceSetupResult<Vec<SuggestedAgent>> {
        let mut agents = Vec::new();
        let count = count as usize;

        match archetype {
            ProjectArchetype::CliTool => {
                // For CLI tools, we typically need fewer agents
                agents.push(SuggestedAgent {
                    name: "cli-developer".to_string(),
                    description: "Develops command-line interface and core functionality"
                        .to_string(),
                    required_capabilities: vec![
                        "cli-development".to_string(),
                        "argument-parsing".to_string(),
                        "file-io".to_string(),
                    ],
                    workload_percentage: if count == 1 { 100.0 } else { 70.0 },
                    depends_on: vec![],
                });

                if count > 1 {
                    agents.push(SuggestedAgent {
                        name: "qa-tester".to_string(),
                        description: "Tests CLI tool across different scenarios and platforms"
                            .to_string(),
                        required_capabilities: vec![
                            "testing".to_string(),
                            "quality-assurance".to_string(),
                        ],
                        workload_percentage: 30.0,
                        depends_on: vec!["cli-developer".to_string()],
                    });
                }

                if count > 2 {
                    agents.push(SuggestedAgent {
                        name: "documentation-writer".to_string(),
                        description: "Creates user documentation and help text".to_string(),
                        required_capabilities: vec![
                            "documentation".to_string(),
                            "technical-writing".to_string(),
                        ],
                        workload_percentage: 20.0,
                        depends_on: vec!["cli-developer".to_string()],
                    });
                    // Adjust percentages
                    agents[0].workload_percentage = 50.0;
                    agents[1].workload_percentage = 30.0;
                }
            }

            ProjectArchetype::Script => {
                // Scripts are very simple - 1-2 agents
                agents.push(SuggestedAgent {
                    name: "script-developer".to_string(),
                    description: "Develops automation scripts and handles core functionality"
                        .to_string(),
                    required_capabilities: vec!["scripting".to_string(), "automation".to_string()],
                    workload_percentage: if count == 1 { 100.0 } else { 80.0 },
                    depends_on: vec![],
                });

                if count > 1 {
                    agents.push(SuggestedAgent {
                        name: "script-reviewer".to_string(),
                        description: "Reviews and validates script functionality".to_string(),
                        required_capabilities: vec![
                            "code-review".to_string(),
                            "testing".to_string(),
                        ],
                        workload_percentage: 20.0,
                        depends_on: vec!["script-developer".to_string()],
                    });
                }
            }

            ProjectArchetype::WebApplication => {
                // Use the original complex agent generation for web apps
                return self.generate_suggested_agents(prd, count as u8).await;
            }

            ProjectArchetype::ApiService => {
                agents.push(SuggestedAgent {
                    name: "api-architect".to_string(),
                    description: "Designs API endpoints, data models, and service architecture"
                        .to_string(),
                    required_capabilities: vec![
                        "api-design".to_string(),
                        "architecture".to_string(),
                        "data-modeling".to_string(),
                    ],
                    workload_percentage: 100.0 / count as f32,
                    depends_on: vec![],
                });

                if count > 1 {
                    agents.push(SuggestedAgent {
                        name: "backend-developer".to_string(),
                        description: "Implements API endpoints and business logic".to_string(),
                        required_capabilities: vec![
                            "backend-development".to_string(),
                            "database-integration".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["api-architect".to_string()],
                    });
                }

                if count > 2 {
                    agents.push(SuggestedAgent {
                        name: "database-specialist".to_string(),
                        description: "Designs and optimizes database schema and queries"
                            .to_string(),
                        required_capabilities: vec![
                            "database-design".to_string(),
                            "performance-optimization".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["api-architect".to_string()],
                    });
                }
            }

            ProjectArchetype::Library => {
                agents.push(SuggestedAgent {
                    name: "library-architect".to_string(),
                    description: "Designs library API and public interfaces".to_string(),
                    required_capabilities: vec![
                        "api-design".to_string(),
                        "library-design".to_string(),
                        "architecture".to_string(),
                    ],
                    workload_percentage: 100.0 / count as f32,
                    depends_on: vec![],
                });

                if count > 1 {
                    agents.push(SuggestedAgent {
                        name: "library-developer".to_string(),
                        description: "Implements library functionality and core features"
                            .to_string(),
                        required_capabilities: vec![
                            "library-development".to_string(),
                            "testing".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["library-architect".to_string()],
                    });
                }

                if count > 2 {
                    agents.push(SuggestedAgent {
                        name: "documentation-specialist".to_string(),
                        description: "Creates comprehensive API documentation and examples"
                            .to_string(),
                        required_capabilities: vec![
                            "technical-documentation".to_string(),
                            "api-documentation".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["library-architect".to_string()],
                    });
                }
            }

            ProjectArchetype::MobileApp => {
                agents.push(SuggestedAgent {
                    name: "mobile-ui-designer".to_string(),
                    description: "Designs mobile user interface and user experience".to_string(),
                    required_capabilities: vec![
                        "ui-design".to_string(),
                        "mobile-design".to_string(),
                        "ux-design".to_string(),
                    ],
                    workload_percentage: 100.0 / count as f32,
                    depends_on: vec![],
                });

                if count > 1 {
                    agents.push(SuggestedAgent {
                        name: "mobile-developer".to_string(),
                        description: "Develops mobile application for target platforms".to_string(),
                        required_capabilities: vec![
                            "mobile-development".to_string(),
                            "platform-integration".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["mobile-ui-designer".to_string()],
                    });
                }

                if count > 2 {
                    agents.push(SuggestedAgent {
                        name: "mobile-qa-tester".to_string(),
                        description: "Tests mobile app on various devices and platforms"
                            .to_string(),
                        required_capabilities: vec![
                            "mobile-testing".to_string(),
                            "device-testing".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["mobile-developer".to_string()],
                    });
                }
            }

            ProjectArchetype::DesktopApp => {
                agents.push(SuggestedAgent {
                    name: "desktop-ui-developer".to_string(),
                    description: "Develops desktop GUI and user interface".to_string(),
                    required_capabilities: vec![
                        "gui-development".to_string(),
                        "desktop-ui".to_string(),
                    ],
                    workload_percentage: 100.0 / count as f32,
                    depends_on: vec![],
                });

                if count > 1 {
                    agents.push(SuggestedAgent {
                        name: "desktop-backend-developer".to_string(),
                        description: "Implements desktop application logic and data handling"
                            .to_string(),
                        required_capabilities: vec![
                            "desktop-development".to_string(),
                            "application-logic".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["desktop-ui-developer".to_string()],
                    });
                }

                if count > 2 {
                    agents.push(SuggestedAgent {
                        name: "platform-integration-specialist".to_string(),
                        description: "Handles OS-specific integrations and packaging".to_string(),
                        required_capabilities: vec![
                            "platform-integration".to_string(),
                            "packaging".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["desktop-backend-developer".to_string()],
                    });
                }
            }

            ProjectArchetype::DataProcessing => {
                agents.push(SuggestedAgent {
                    name: "data-architect".to_string(),
                    description: "Designs data processing pipeline architecture".to_string(),
                    required_capabilities: vec![
                        "data-architecture".to_string(),
                        "pipeline-design".to_string(),
                    ],
                    workload_percentage: 100.0 / count as f32,
                    depends_on: vec![],
                });

                if count > 1 {
                    agents.push(SuggestedAgent {
                        name: "data-engineer".to_string(),
                        description: "Implements data processing logic and ETL operations"
                            .to_string(),
                        required_capabilities: vec![
                            "data-engineering".to_string(),
                            "etl-development".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["data-architect".to_string()],
                    });
                }

                if count > 2 {
                    agents.push(SuggestedAgent {
                        name: "data-quality-specialist".to_string(),
                        description: "Ensures data quality and pipeline monitoring".to_string(),
                        required_capabilities: vec![
                            "data-quality".to_string(),
                            "monitoring".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["data-engineer".to_string()],
                    });
                }
            }

            ProjectArchetype::Generic => {
                // Fallback agent generation for unclassifiable projects
                agents.push(SuggestedAgent {
                    name: "project-lead".to_string(),
                    description: "Leads project development and coordinates team efforts"
                        .to_string(),
                    required_capabilities: vec![
                        "project-management".to_string(),
                        "general-development".to_string(),
                    ],
                    workload_percentage: 100.0 / count as f32,
                    depends_on: vec![],
                });

                if count > 1 {
                    agents.push(SuggestedAgent {
                        name: "developer".to_string(),
                        description: "Implements project functionality and features".to_string(),
                        required_capabilities: vec!["general-development".to_string()],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["project-lead".to_string()],
                    });
                }

                if count > 2 {
                    agents.push(SuggestedAgent {
                        name: "qa-specialist".to_string(),
                        description: "Ensures quality and tests project deliverables".to_string(),
                        required_capabilities: vec![
                            "testing".to_string(),
                            "quality-assurance".to_string(),
                        ],
                        workload_percentage: 100.0 / count as f32,
                        depends_on: vec!["developer".to_string()],
                    });
                }
            }
        }

        // Ensure we return exactly the requested number of agents
        agents.truncate(count);
        Ok(agents)
    }

    async fn generate_suggested_agents(
        &self,
        _prd: &PrdDocument,
        count: u8,
    ) -> WorkspaceSetupResult<Vec<SuggestedAgent>> {
        let mut agents = Vec::new();
        let workload_per_agent = 100.0 / count as f32;

        // Define comprehensive agent roles for web applications
        let web_app_agents = vec![
            (
                "project-manager",
                "Coordinates overall project execution and manages task assignments",
                vec!["project-management", "coordination"],
                vec![],
            ),
            (
                "backend-developer",
                "Implements server-side logic, APIs, and database integration",
                vec!["backend-development", "api-design"],
                vec!["project-manager"],
            ),
            (
                "frontend-developer",
                "Creates user interfaces and client-side application logic",
                vec!["frontend-development", "ui-design"],
                vec!["project-manager"],
            ),
            (
                "database-architect",
                "Designs database schema, optimizes queries, and manages data models",
                vec!["database-design", "sql", "data-modeling"],
                vec!["backend-developer"],
            ),
            (
                "devops-engineer",
                "Manages deployment, infrastructure, and CI/CD pipelines",
                vec!["devops", "cloud-infrastructure", "ci-cd"],
                vec!["backend-developer"],
            ),
            (
                "qa-engineer",
                "Designs and executes test plans, ensures quality assurance",
                vec!["testing", "quality-assurance", "automation"],
                vec!["frontend-developer", "backend-developer"],
            ),
            (
                "ui-ux-designer",
                "Creates user interface designs and user experience flows",
                vec!["ui-design", "ux-design", "user-research"],
                vec!["project-manager"],
            ),
            (
                "security-specialist",
                "Implements security measures and conducts security audits",
                vec!["security", "penetration-testing", "compliance"],
                vec!["backend-developer"],
            ),
        ];

        // Add agents up to the requested count
        for i in 0..count.min(web_app_agents.len() as u8) {
            let (name, description, capabilities, dependencies) = &web_app_agents[i as usize];
            agents.push(SuggestedAgent {
                name: name.to_string(),
                description: description.to_string(),
                required_capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
                workload_percentage: workload_per_agent,
                depends_on: dependencies.iter().map(|s| s.to_string()).collect(),
            });
        }

        // If we need more agents than our predefined roles, add generic developers
        while agents.len() < count as usize {
            let agent_number = agents.len() + 1;
            agents.push(SuggestedAgent {
                name: format!("developer-{agent_number}"),
                description: format!("Additional development resource #{agent_number}"),
                required_capabilities: vec![
                    "general-development".to_string(),
                    "problem-solving".to_string(),
                ],
                workload_percentage: workload_per_agent,
                depends_on: vec!["project-manager".to_string()],
            });
        }

        Ok(agents)
    }

    /// Generate workflow steps based on project archetype and suggested agents
    fn generate_workflow_steps(
        &self,
        archetype: &ProjectArchetype,
        suggested_agents: &[SuggestedAgent],
    ) -> Vec<String> {
        let mut steps = Vec::new();

        match archetype {
            ProjectArchetype::WebApplication => {
                steps.push("1. Project setup and initial planning - project-manager coordinates team kickoff".to_string());
                steps.push("2. Database design and schema creation - database-architect designs core data models".to_string());
                steps.push("3. UI/UX design and wireframing - ui-ux-designer creates user interface mockups".to_string());
                steps.push("4. Backend API development - backend-developer implements server-side logic and APIs".to_string());
                steps.push("5. Frontend application development - frontend-developer builds user interface components".to_string());
                steps.push("6. Integration and system testing - qa-engineer verifies all components work together".to_string());
                steps.push("7. Security audit and hardening - security-specialist reviews and secures the application".to_string());
                steps.push("8. DevOps setup and deployment - devops-engineer configures CI/CD and production environment".to_string());
            }
            ProjectArchetype::CliTool => {
                steps.push("1. Command-line interface design - cli-developer designs argument parsing and commands".to_string());
                steps.push("2. Core functionality implementation - cli-developer implements main business logic".to_string());
                steps.push(
                    "3. Error handling and validation - cli-developer adds robust error handling"
                        .to_string(),
                );
                steps.push("4. Testing and documentation - cli-developer creates tests and user documentation".to_string());
                steps.push("5. Package and distribution - cli-developer prepares for distribution and installation".to_string());
            }
            ProjectArchetype::ApiService => {
                steps.push("1. API design and specification - api-architect designs endpoint specifications".to_string());
                steps.push("2. Data model and persistence layer - backend-developer implements data access layer".to_string());
                steps.push("3. API endpoint implementation - backend-developer implements REST/GraphQL endpoints".to_string());
                steps.push("4. Authentication and authorization - security-specialist implements access control".to_string());
                steps.push("5. API testing and validation - qa-engineer creates comprehensive API test suite".to_string());
                steps.push("6. Documentation and deployment - devops-engineer sets up API documentation and deployment".to_string());
            }
            ProjectArchetype::MobileApp => {
                steps.push(
                    "1. Mobile UI/UX design - mobile-ui-designer creates platform-specific designs"
                        .to_string(),
                );
                steps.push("2. Mobile app development - mobile-developer implements app for target platforms".to_string());
                steps.push("3. Platform integration - mobile-developer integrates with platform-specific features".to_string());
                steps.push("4. Device testing - mobile-qa-tester tests on various devices and screen sizes".to_string());
                steps.push(
                    "5. App store preparation - mobile-developer prepares for app store submission"
                        .to_string(),
                );
            }
            ProjectArchetype::Library => {
                steps.push(
                    "1. Library architecture design - library-architect defines API and structure"
                        .to_string(),
                );
                steps.push(
                    "2. Core implementation - library-developer implements main functionality"
                        .to_string(),
                );
                steps.push(
                    "3. API documentation - documentation-writer creates comprehensive API docs"
                        .to_string(),
                );
                steps.push(
                    "4. Testing and examples - library-developer creates tests and usage examples"
                        .to_string(),
                );
                steps.push(
                    "5. Package and publish - library-developer prepares for package distribution"
                        .to_string(),
                );
            }
            ProjectArchetype::DataProcessing => {
                steps.push(
                    "1. Data pipeline architecture - data-architect designs processing pipeline"
                        .to_string(),
                );
                steps.push("2. Data ingestion implementation - data-engineer implements data input mechanisms".to_string());
                steps.push("3. Processing logic development - data-engineer implements transformation logic".to_string());
                steps.push("4. Output and storage integration - data-engineer implements data output and storage".to_string());
                steps.push("5. Performance optimization and monitoring - data-engineer optimizes for scale".to_string());
            }
            ProjectArchetype::DesktopApp => {
                steps.push(
                    "1. Desktop UI design - desktop-ui-developer designs application interface"
                        .to_string(),
                );
                steps.push("2. Application logic implementation - desktop-backend-developer implements business logic".to_string());
                steps.push(
                    "3. UI integration - desktop-ui-developer connects UI with backend logic"
                        .to_string(),
                );
                steps.push("4. Cross-platform testing - desktop-qa-tester tests on different operating systems".to_string());
                steps.push("5. Installation package creation - desktop-backend-developer creates installers".to_string());
            }
            ProjectArchetype::Script => {
                steps.push(
                    "1. Script requirements analysis - developer analyzes automation requirements"
                        .to_string(),
                );
                steps.push(
                    "2. Core script implementation - developer implements main automation logic"
                        .to_string(),
                );
                steps.push(
                    "3. Error handling and logging - developer adds robust error handling"
                        .to_string(),
                );
                steps.push(
                    "4. Testing and validation - developer creates test scenarios".to_string(),
                );
                steps.push(
                    "5. Documentation and deployment - developer creates usage documentation"
                        .to_string(),
                );
            }
            ProjectArchetype::Generic => {
                // Generate generic steps based on available agents
                if suggested_agents.iter().any(|a| a.name.contains("manager")) {
                    steps.push("1. Project planning and requirements analysis - project-manager defines scope and goals".to_string());
                }
                if suggested_agents
                    .iter()
                    .any(|a| a.name.contains("developer"))
                {
                    steps.push(
                        "2. Core implementation - developers implement main functionality"
                            .to_string(),
                    );
                }
                if suggested_agents
                    .iter()
                    .any(|a| a.name.contains("qa") || a.name.contains("test"))
                {
                    steps.push("3. Quality assurance and testing - qa-engineer creates and executes test plans".to_string());
                }
                if suggested_agents
                    .iter()
                    .any(|a| a.name.contains("devops") || a.name.contains("deploy"))
                {
                    steps.push("4. Deployment and operations - devops-engineer handles deployment and monitoring".to_string());
                }

                // Add generic fallback steps if no specific agents found
                if steps.is_empty() {
                    steps.push("1. Project setup and initial development".to_string());
                    steps.push("2. Implementation of core functionality".to_string());
                    steps.push("3. Testing and quality assurance".to_string());
                    steps.push("4. Deployment and finalization".to_string());
                }
            }
        }

        steps
    }
}

/// Workspace context for stateful workflow orchestration
///
/// This stores the state between MCP function calls within a single workspace setup workflow,
/// enabling functions to share data and build upon previous results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContext {
    pub workspace_id: String,
    pub version: i32,
    pub prd_content: Option<String>,
    pub workflow_data: Option<AgenticWorkflowDescription>,
    pub registered_agents: Vec<AgentRegistration>,
    pub generated_files: Vec<GeneratedFileMetadata>,
    pub manifest_data: Option<WorkspaceManifest>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Metadata for files generated during workspace setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFileMetadata {
    pub path: String,
    pub description: String,
    pub ai_tool_type: AiToolType,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
}

impl WorkspaceContext {
    /// Create a new workspace context
    pub fn new(workspace_id: String) -> Self {
        let now = Utc::now();
        Self {
            workspace_id,
            version: 1,
            prd_content: None,
            workflow_data: None,
            registered_agents: Vec::new(),
            generated_files: Vec::new(),
            manifest_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update PRD content and increment version
    pub fn update_prd(&mut self, prd_content: String) {
        self.prd_content = Some(prd_content);
        self.increment_version();
    }

    /// Update workflow data and increment version
    pub fn update_workflow(&mut self, workflow_data: AgenticWorkflowDescription) {
        self.workflow_data = Some(workflow_data);
        self.increment_version();
    }

    /// Register an agent and increment version
    pub fn register_agent(&mut self, agent: AgentRegistration) {
        self.registered_agents.push(agent);
        self.increment_version();
    }

    /// Add generated file metadata and increment version
    pub fn add_generated_file(&mut self, file_metadata: GeneratedFileMetadata) {
        self.generated_files.push(file_metadata);
        self.increment_version();
    }

    /// Update manifest and increment version
    pub fn update_manifest(&mut self, manifest: WorkspaceManifest) {
        self.manifest_data = Some(manifest);
        self.increment_version();
    }

    /// Get recommended agent count from workflow data
    pub fn get_recommended_agent_count(&self) -> u32 {
        self.workflow_data
            .as_ref()
            .map(|w| w.recommended_agent_count)
            .unwrap_or(0)
    }

    /// Check if workflow data is available
    pub fn has_workflow_data(&self) -> bool {
        self.workflow_data.is_some()
    }

    /// Check if PRD is available
    pub fn has_prd(&self) -> bool {
        self.prd_content.is_some()
    }

    /// Private helper to increment version and update timestamp
    fn increment_version(&mut self) {
        self.version += 1;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_tool_type_display() {
        assert_eq!(AiToolType::ClaudeCode.to_string(), "claude-code");
    }

    #[test]
    fn test_prd_parsing() {
        let prd_content = r#"
# Test Project

## Overview
Test project overview.

## Objectives
- Build something
- Test it

## Technical Requirements
- React frontend
- Node.js backend

## User Stories
- User can login
- User can logout
        "#;

        let prd = PrdDocument::from_content(prd_content).unwrap();
        assert_eq!(prd.title, "Test Project");
        assert!(prd.is_valid());
        assert_eq!(prd.objectives.len(), 2);
        assert_eq!(prd.technical_requirements.len(), 2);
        assert_eq!(prd.user_stories.len(), 2);
    }

    #[tokio::test]
    async fn test_workspace_setup_service() {
        let service = WorkspaceSetupService::new();

        // Test setup instructions
        let response = service
            .get_setup_instructions(AiToolType::ClaudeCode)
            .await
            .unwrap();
        assert_eq!(response.status, ResponseStatus::Success);
        assert_eq!(response.payload.ai_tool_type, AiToolType::ClaudeCode);
    }

    // Helper function to create mock PRD documents for testing
    fn create_test_prd(title: &str, overview: &str, tech_requirements: &[&str]) -> PrdDocument {
        PrdDocument {
            title: title.to_string(),
            overview: Some(overview.to_string()),
            objectives: vec!["Test objective".to_string()],
            user_stories: vec!["Test user story".to_string()],
            technical_requirements: tech_requirements.iter().map(|s| s.to_string()).collect(),
            success_criteria: vec![],
            constraints: vec![],
            timeline: None,
            raw_content: format!(
                "# {}\n\n## Overview\n{}\n\n## Technical Requirements\n{}",
                title,
                overview,
                tech_requirements.join("\n- ")
            ),
            validation_errors: vec![],
        }
    }

    // Unit tests for classify_project_archetype function
    mod archetype_classification_tests {
        use super::*;

        #[test]
        fn test_cli_tool_classification() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "Markdown Converter CLI",
                "A command-line tool for converting markdown files",
                &["cli", "command-line interface", "file conversion"],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::CliTool);
        }

        #[test]
        fn test_web_application_classification() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "E-commerce Web App",
                "A full-stack web application with React frontend and Node.js backend",
                &[
                    "react", "frontend", "node.js", "backend", "database", "html", "css",
                ],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::WebApplication);
        }

        #[test]
        fn test_api_service_classification() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "REST API Service",
                "A microservice providing REST API endpoints for user management",
                &[
                    "api",
                    "rest",
                    "microservice",
                    "endpoints",
                    "json",
                    "database",
                ],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::ApiService);
        }

        #[test]
        fn test_mobile_app_classification() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "iOS Shopping App",
                "A native mobile application for iOS and Android platforms",
                &["ios", "android", "mobile app", "swift", "kotlin"],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::MobileApp);
        }

        #[test]
        fn test_desktop_app_classification() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "Desktop Text Editor",
                "A cross-platform desktop application with GUI",
                &["desktop", "gui", "electron", "cross-platform"],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::DesktopApp);
        }

        #[test]
        fn test_library_classification() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "HTTP Client Library",
                "A reusable library for making HTTP requests",
                &[
                    "library",
                    "sdk",
                    "reusable components",
                    "package",
                    "framework",
                ],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::Library);
        }

        #[test]
        fn test_data_processing_classification() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "ETL Pipeline",
                "Data processing pipeline for analytics",
                &[
                    "etl",
                    "data processing",
                    "pipeline",
                    "spark",
                    "kafka",
                    "analytics",
                ],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::DataProcessing);
        }

        #[test]
        fn test_script_classification() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "Deployment Automation",
                "Automated deployment script for CI/CD",
                &["automation", "script", "batch", "deployment"],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::Script);
        }

        // Boundary case tests - these are critical for robustness

        #[test]
        fn test_api_for_web_app_classified_as_api_service() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "API for Web Application",
                "RESTful API backend that serves our main web application",
                &["api", "rest", "json", "database", "web app context"],
            );

            // Thanks to improved ordering, this should be classified as ApiService
            // despite mentioning "web app"
            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::ApiService);
        }

        #[test]
        fn test_cli_tool_with_api_classified_as_cli_tool() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "CLI Tool with Network Access",
                "A command-line tool that interacts with external web services",
                &[
                    "cli",
                    "command-line",
                    "tool",
                    "http requests",
                    "file processing",
                    "json parsing",
                ],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::CliTool);
        }

        #[test]
        fn test_responsive_web_app_for_mobile_is_web_application() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "Responsive Web Design",
                "A web application with mobile-responsive design for mobile browsers",
                &[
                    "web",
                    "responsive",
                    "mobile",
                    "html",
                    "css",
                    "javascript",
                    "frontend",
                    "backend",
                ],
            );

            // Should be WebApplication, not MobileApp, due to presence of "web"
            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::WebApplication);
        }

        #[test]
        fn test_web_app_full_stack_classification() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "Full-Stack Application",
                "A complete web application with both frontend and backend",
                &["full-stack", "react", "node.js"],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::WebApplication);
        }

        // Edge case tests

        #[test]
        fn test_vague_prd_classified_as_generic() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "Some Project",
                "We want to build something cool",
                &["modern technology", "innovative solution", "user-friendly"],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::Generic);
        }

        #[test]
        fn test_empty_requirements_classified_as_generic() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "Undefined Project",
                "Project with no clear technical direction",
                &[],
            );

            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::Generic);
        }

        #[test]
        fn test_mixed_signals_prioritizes_specific_archetype() {
            let service = WorkspaceSetupService::new();
            let prd = create_test_prd(
                "Multi-Purpose Tool",
                "A desktop application that also provides an API",
                &["desktop", "gui", "api", "electron"],
            );

            // Desktop comes first in priority order, so should be DesktopApp
            let archetype = service.classify_project_archetype(&prd);
            assert_eq!(archetype, ProjectArchetype::DesktopApp);
        }
    }
}
