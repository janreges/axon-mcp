use anyhow::Result;
use clap::Parser;
use handlebars::Handlebars;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use task_core::{PrdDocument, workspace_setup::{*, SuggestedAgent}};
use tracing::{info, error, debug};

mod poc_test;

/// Workspace Orchestrator for Dynamic Agent Team Generation
/// 
/// Implements Pro model recommendations for intelligent agent orchestration
/// using R.I.C.H. prompting patterns and dynamic team composition.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the PRD (Product Requirements Document) file
    #[arg(short, long, required_unless_present = "poc_test")]
    prd_path: Option<String>,
    
    /// MCP server URL
    #[arg(short, long, default_value = "http://localhost:8080")]
    mcp_url: String,
    
    /// Template file path
    #[arg(short, long, default_value = "templates/CLAUDE.md.hbs")]
    template_path: String,
    
    /// Output path for generated control agent prompt
    #[arg(short, long, default_value = "output/CLAUDE.md")]
    output_path: String,
    
    /// Run POC test with mock data instead of calling MCP server
    #[arg(long)]
    poc_test: bool,
}

/// Configuration structure as recommended by Pro model
#[derive(Debug, Clone)]
struct OrchestratorConfig {
    mcp_server_url: String,
    template_path: String,
    prd_path: Option<String>,
    output_path: String,
    poc_test: bool,
}

impl From<Args> for OrchestratorConfig {
    fn from(args: Args) -> Self {
        Self {
            mcp_server_url: args.mcp_url,
            template_path: args.template_path,
            prd_path: args.prd_path,
            output_path: args.output_path,
            poc_test: args.poc_test,
        }
    }
}

/// MCP Client for communicating with our MCP server
#[derive(Debug, Clone)]
struct McpClient {
    client: Client,
    base_url: String,
}

impl McpClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }
    
    /// Call get_agentic_workflow_description MCP function
    pub async fn get_agentic_workflow_description(&self, prd: &PrdDocument) -> Result<AgenticWorkflowDescription> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "get_agentic_workflow_description",
            "params": {
                "prd_content": serde_json::to_string(prd)?
            }
        });
        
        debug!("Calling MCP function: get_agentic_workflow_description");
        let response = self.client
            .post(&format!("{}/mcp", self.base_url))
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            error!("MCP call failed with status: {}", response.status());
            return Err(anyhow::anyhow!("MCP call failed: {}", response.status()));
        }
        
        let response_json: serde_json::Value = response.json().await?;
        
        if let Some(error) = response_json.get("error") {
            error!("MCP returned error: {}", error);
            return Err(anyhow::anyhow!("MCP error: {}", error));
        }
        
        let result = response_json
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in MCP response"))?;
            
        let workflow: AgenticWorkflowDescription = serde_json::from_value(result.clone())?;
        info!("Successfully received workflow description with {} agents", workflow.suggested_agents.len());
        
        Ok(workflow)
    }
}

/// Enhanced Template Engine implementing Pro model R.I.C.H. pattern
/// 
/// R.I.C.H. Pattern:
/// - Role-Specific: Each agent gets precise role definition
/// - Imperative: Direct commands with MUST/SHALL imperatives
/// - Contextual: Full project context embedded in prompts
/// - Handoff: Clear coordination and communication protocols
#[derive(Debug)]
struct ClaudemdTemplate {
    engine: Handlebars<'static>,
    template_name: String,
}

impl ClaudemdTemplate {
    pub fn new(template_str: &str) -> Result<Self> {
        let mut engine = Handlebars::new();
        let template_name = "claude_md".to_string();
        
        // Register template with handlebars
        engine.register_template_string(&template_name, template_str)
            .map_err(|e| anyhow::anyhow!("Failed to register template: {}", e))?;
            
        Ok(Self { engine, template_name })
    }
    
    /// Render template with enhanced agent data following R.I.C.H. pattern
    pub fn render(&self, workflow: &AgenticWorkflowDescription, prd: &PrdDocument) -> Result<String> {
        // Enhanced template data with R.I.C.H. pattern elements
        let template_data = json!({
            "project_title": prd.title,
            "project_overview": prd.overview.as_ref().unwrap_or(&"No overview provided".to_string()),
            "recommended_agent_count": workflow.recommended_agent_count,
            "task_decomposition_strategy": workflow.task_decomposition_strategy,
            
            // Enhanced agent data with R.I.C.H. prompting
            "agents": workflow.suggested_agents.iter().map(|agent| {
                json!({
                    "name": agent.name,
                    "role_description": agent.description,
                    
                    // R.I.C.H. Enhanced Prompt Components
                    "rich_prompt": self.create_rich_agent_prompt(agent, prd, &workflow.coordination_patterns),
                    
                    // JSON structure for Task tool (Pro model recommendation)
                    "task_tool_json": json!({
                        "agent_role_and_task": self.create_rich_agent_prompt(agent, prd, &workflow.coordination_patterns)
                    }).to_string()
                })
            }).collect::<Vec<_>>(),
            
            "coordination_patterns": workflow.coordination_patterns,
            "estimated_timeline": 4, // Default timeline in weeks
        });
        
        let rendered = self.engine.render(&self.template_name, &template_data)
            .map_err(|e| anyhow::anyhow!("Template rendering failed: {}", e))?;
            
        debug!("Template rendered successfully, {} characters", rendered.len());
        Ok(rendered)
    }
    
    /// Create R.I.C.H. pattern agent prompt
    /// 
    /// Following Pro model recommendations:
    /// - Role: Specific persona and expertise
    /// - Instruction: Imperative commands with structure
    /// - Context: Full project understanding 
    /// - Handoff: Communication protocols with other agents
    pub fn create_rich_agent_prompt(&self, agent: &SuggestedAgent, prd: &PrdDocument, coordination_patterns: &[String]) -> String {
        format!(
            r#"# {agent_role} - Project: {project_title}

## 🎯 YOUR ROLE (Role-Specific)
You are a **{agent_role}** with deep expertise in your domain. Your primary responsibility is: {agent_description}

## 📋 PROJECT CONTEXT (Contextual)
**Project:** {project_title}
**Overview:** {project_overview}
**Your Focus Areas:** Based on the project requirements, you MUST focus on implementing the technical requirements that align with your expertise.

**Key Technical Requirements:** 
{technical_requirements}

## 🚨 CRITICAL INSTRUCTIONS (Imperative)
1. **START IMMEDIATELY**: Begin your work without waiting for other agents
2. **BE PROACTIVE**: Take initiative in your area of expertise  
3. **COMMUNICATE ACTIVELY**: Use the MCP messaging system for coordination
4. **DELIVER QUALITY**: Your work represents the foundation for the entire project

## 🤝 TEAM COORDINATION (Handoff)
You are part of a {agent_count}-member specialized team working on this project.

**Coordination Patterns:**
{coordination_info}

**COMMUNICATION PROTOCOL:**
- **For blockers:** Send `blocker` type message to relevant team member
- **For questions:** Send `question` type message with specific queries
- **For handoffs:** Send `handoff` type message with deliverables and next steps
- **For updates:** Send `comment` type message to keep team informed

## 💼 SUCCESS CRITERIA
Your work is successful when:
1. All technical requirements in your domain are implemented
2. Code is tested and documented
3. Integration points with other agents are clearly defined
4. Any blockers or dependencies are communicated immediately

**REMEMBER:** You are a senior expert in your field. Act with confidence, take ownership, and deliver exceptional results."#,
            agent_role = agent.name,
            project_title = prd.title,
            agent_description = agent.description,
            project_overview = prd.overview.as_ref().unwrap_or(&"No overview provided".to_string()),
            technical_requirements = prd.technical_requirements.join("\n- "),
            agent_count = "multiple", // Could be enhanced with actual count
            coordination_info = coordination_patterns.join("\n- ")
        )
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for observability (Pro model recommendation)
    tracing_subscriber::fmt::init();
    
    let args = Args::parse();
    let config = OrchestratorConfig::from(args);
    
    info!("🚀 Starting Workspace Orchestrator POC");
    info!("Configuration: {:?}", config);
    
    // Check if this is POC test mode
    if config.poc_test {
        info!("🧪 Running POC test with mock data");
        return poc_test::run_poc_test().await;
    }
    
    // Step 1: Load PRD (Pro model POC workflow)
    let prd_path = config.prd_path.as_ref().unwrap();
    info!("📄 Loading PRD from: {}", prd_path);
    let prd_content = fs::read_to_string(prd_path)
        .map_err(|e| anyhow::anyhow!("Failed to read PRD file '{}': {}", prd_path, e))?;
        
    let prd = PrdDocument::from_content(&prd_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse PRD: {}", e))?;
        
    info!("✅ PRD loaded successfully: '{}'", prd.title);
    info!("   Valid: {}, Objectives: {}, User Stories: {}, Tech Requirements: {}", 
          prd.is_valid(), prd.objectives.len(), prd.user_stories.len(), prd.technical_requirements.len());
    
    // Step 2: Call MCP service for agent workflow description  
    info!("🤖 Calling MCP service for agent workflow analysis");
    let mcp_client = McpClient::new(&config.mcp_server_url);
    let workflow = mcp_client.get_agentic_workflow_description(&prd).await?;
    
    info!("✅ Workflow analysis complete:");
    info!("   Recommended Agents: {}", workflow.recommended_agent_count);
    info!("   Generated Agents: {}", workflow.suggested_agents.len());
    info!("   Strategy: {}", workflow.task_decomposition_strategy);
    
    // Log each agent for visibility
    for (i, agent) in workflow.suggested_agents.iter().enumerate() {
        info!("   Agent {}: {} - {}", i+1, agent.name, agent.description);
    }
    
    // Step 3: Load and render template with R.I.C.H. pattern
    info!("📝 Loading template from: {}", config.template_path);
    let template_content = fs::read_to_string(&config.template_path)
        .map_err(|e| anyhow::anyhow!("Failed to read template file '{}': {}", config.template_path, e))?;
        
    let template = ClaudemdTemplate::new(&template_content)?;
    let rendered_prompt = template.render(&workflow, &prd)?;
    
    // Step 4: Output generated prompt (POC validation step)
    info!("💾 Writing generated control agent prompt to: {}", config.output_path);
    
    // Ensure output directory exists
    if let Some(parent) = std::path::Path::new(&config.output_path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("Failed to create output directory: {}", e))?;
    }
    
    fs::write(&config.output_path, &rendered_prompt)
        .map_err(|e| anyhow::anyhow!("Failed to write output file '{}': {}", config.output_path, e))?;
    
    // Step 5: POC Success Summary
    info!("🎉 POC COMPLETED SUCCESSFULLY!");
    info!("════════════════════════════════════════");
    info!("📊 RESULTS SUMMARY:");
    info!("   Project: {}", prd.title);
    info!("   Archetype: Dynamically classified (vs. hardcoded 8 agents)");
    info!("   Recommended Agents: {} (optimized for project complexity)", workflow.recommended_agent_count);
    info!("   Generated Prompt: {} characters", rendered_prompt.len());
    info!("   R.I.C.H. Pattern: ✅ Implemented");
    info!("   Template System: ✅ Handlebars with dynamic data");
    info!("   Task Tool JSON: ✅ Structured prompts ready");
    info!("════════════════════════════════════════");
    info!("📁 Generated control agent prompt saved to: {}", config.output_path);
    info!("🔍 Review the generated prompt to validate R.I.C.H. pattern implementation");
    
    println!("\n--- GENERATED PROMPT PREVIEW (first 500 chars) ---");
    println!("{}", rendered_prompt.chars().take(500).collect::<String>());
    if rendered_prompt.len() > 500 {
        println!("... (truncated, see {} for full content)", config.output_path);
    }
    println!("--- END PREVIEW ---\n");
    
    Ok(())
}
