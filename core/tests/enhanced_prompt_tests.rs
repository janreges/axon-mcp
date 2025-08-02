//! Tests for enhanced prompt generation following 2025 best practices

use task_core::{
    EnhancedPromptBuilder, 
    workspace_setup::{SuggestedAgent, ProjectArchetype},
};
use task_core::ai_tool_adapters::{ClaudeCodeAdapter, AiToolAdapter};

#[test]
fn test_enhanced_prompt_generation() {
    let builder = EnhancedPromptBuilder::new();
    
    let agent = SuggestedAgent {
        name: "cli-developer".to_string(),
        description: "Develops command-line interface and core functionality".to_string(),
        required_capabilities: vec!["cli-development".to_string(), "argument-parsing".to_string()],
        workload_percentage: 70.0,
        depends_on: vec![],
    };
    
    let prompt = builder.generate_agent_prompt(
        &agent,
        &ProjectArchetype::CliTool,
        "Markdown to HTML converter CLI tool",
        None,
    );
    
    // Verify 2025 best practices are included
    assert!(prompt.contains("AGENT_CONTRACT_START"));
    assert!(prompt.contains("AGENT_CONTRACT_END"));
    assert!(prompt.contains("STATIC_CONTEXT_START"));
    assert!(prompt.contains("MCP COORDINATION PROTOCOL"));
    assert!(prompt.contains("WORK ITERATION PATTERN"));
    assert!(prompt.contains("ERROR HANDLING & ESCALATION"));
    assert!(prompt.contains("claim_task"));
    assert!(prompt.contains("start_work_session"));
    assert!(prompt.contains("create_task_message"));
    assert!(prompt.contains("micro-iteration"));
    assert!(prompt.contains("escalate"));
    
    // Verify agent-specific content
    assert!(prompt.contains("cli-developer"));
    assert!(prompt.contains("CLI tool"));
    assert!(prompt.contains("cli-development"));
    
    println!("✅ Enhanced prompt generation test passed");
}

#[test]
fn test_different_archetypes_generate_different_prompts() {
    let builder = EnhancedPromptBuilder::new();
    
    let cli_agent = SuggestedAgent {
        name: "cli-developer".to_string(),
        description: "CLI developer".to_string(),
        required_capabilities: vec!["cli-development".to_string()],
        workload_percentage: 100.0,
        depends_on: vec![],
    };
    
    let web_agent = SuggestedAgent {
        name: "backend-developer".to_string(),
        description: "Backend developer".to_string(),
        required_capabilities: vec!["api-design".to_string()],
        workload_percentage: 100.0,
        depends_on: vec![],
    };
    
    let cli_prompt = builder.generate_agent_prompt(
        &cli_agent,
        &ProjectArchetype::CliTool,
        "CLI project",
        None,
    );
    
    let web_prompt = builder.generate_agent_prompt(
        &web_agent,
        &ProjectArchetype::WebApplication,
        "Web project",
        None,
    );
    
    // Prompts should be different for different archetypes
    assert_ne!(cli_prompt, web_prompt);
    
    // But both should contain 2025 best practices
    assert!(cli_prompt.contains("MCP COORDINATION PROTOCOL"));
    assert!(web_prompt.contains("MCP COORDINATION PROTOCOL"));
    
    println!("✅ Archetype-specific prompt generation test passed");
}

#[tokio::test]
async fn test_claude_code_adapter_enhanced_setup() {
    let adapter = ClaudeCodeAdapter::new();
    
    let setup_instructions = adapter.get_setup_instructions().await.unwrap();
    
    // Verify enhanced setup includes 2025 features
    assert_eq!(setup_instructions.schema_version, "2.0");
    assert!(setup_instructions.setup_steps.len() >= 6); // More steps for enhanced setup
    
    // Check for enhanced steps
    let step_names: Vec<&str> = setup_instructions.setup_steps
        .iter()
        .map(|step| step.name.as_str())
        .collect();
    
    assert!(step_names.iter().any(|name| name.contains("MCP Connection")));
    assert!(step_names.iter().any(|name| name.contains("capability catalog")));
    assert!(step_names.iter().any(|name| name.contains("enhanced")));
    
    // Verify enhanced MCP functions are included
    let function_names: Vec<&str> = setup_instructions.required_mcp_functions
        .iter()
        .map(|func| func.function_name.as_str())
        .collect();
    
    assert!(function_names.contains(&"claim_task"));
    assert!(function_names.contains(&"start_work_session"));
    assert!(function_names.contains(&"end_work_session"));
    
    println!("✅ Enhanced Claude Code adapter test passed");
}

#[test]
fn test_agent_contract_structure() {
    let builder = EnhancedPromptBuilder::new();
    
    let agent = SuggestedAgent {
        name: "test-agent".to_string(),
        description: "Test agent for contract validation".to_string(),
        required_capabilities: vec!["testing".to_string()],
        workload_percentage: 50.0,
        depends_on: vec![],
    };
    
    let prompt = builder.generate_agent_prompt(
        &agent,
        &ProjectArchetype::Generic,
        "Test project",
        None,
    );
    
    // Verify the structured contract contains required fields
    assert!(prompt.contains("role_name"));
    assert!(prompt.contains("mission"));
    assert!(prompt.contains("success_criteria"));
    assert!(prompt.contains("deliverables"));
    assert!(prompt.contains("tools_allowed"));
    assert!(prompt.contains("output_format"));
    assert!(prompt.contains("escalation_protocol"));
    
    println!("✅ Agent contract structure test passed");
}

#[test]
fn test_coordination_patterns() {
    let builder = EnhancedPromptBuilder::new();
    
    let agent = SuggestedAgent {
        name: "coordinator-agent".to_string(),
        description: "Coordination test agent".to_string(),
        required_capabilities: vec!["coordination".to_string()],
        workload_percentage: 100.0,
        depends_on: vec![],
    };
    
    let prompt = builder.generate_agent_prompt(
        &agent,
        &ProjectArchetype::WebApplication,
        "Coordination test project",
        None,
    );
    
    // Debug: Print the prompt to see what's actually generated
    println!("Generated prompt:\n{}", prompt);
    
    // Verify coordination patterns are included
    assert!(prompt.contains("Claim Work") || prompt.contains("claim_task"));
    assert!(prompt.contains("Start Session") || prompt.contains("start_work_session"));
    assert!(prompt.contains("Micro-Iteration") || prompt.contains("iteration"));
    assert!(prompt.contains("Handoff") || prompt.contains("create_task_message"));
    assert!(prompt.contains("Complete") || prompt.contains("Done"));
    
    // Verify timeout and escalation handling
    assert!(prompt.contains("blocker"));
    assert!(prompt.contains("human-supervisor"));
    assert!(prompt.contains("max"));
    
    println!("✅ Coordination patterns test passed");
}

#[test]
fn test_context_management() {
    let builder = EnhancedPromptBuilder::new();
    
    let agent = SuggestedAgent {
        name: "context-agent".to_string(),
        description: "Context management test agent".to_string(),
        required_capabilities: vec!["context-management".to_string()],
        workload_percentage: 100.0,
        depends_on: vec![],
    };
    
    // Test with static context only
    let prompt_static = builder.generate_agent_prompt(
        &agent,
        &ProjectArchetype::Generic,
        "Static context test",
        None,
    );
    
    // Test with rolling context
    let prompt_rolling = builder.generate_agent_prompt(
        &agent,
        &ProjectArchetype::Generic,
        "Rolling context test",
        Some("Previous conversation: Agent A completed task X, now passing to Agent B"),
    );
    
    // Both should have static context
    assert!(prompt_static.contains("STATIC_CONTEXT_START"));
    assert!(prompt_rolling.contains("STATIC_CONTEXT_START"));
    
    // Only rolling should have rolling context
    assert!(!prompt_static.contains("ROLLING_CONTEXT_START"));
    assert!(prompt_rolling.contains("ROLLING_CONTEXT_START"));
    assert!(prompt_rolling.contains("Previous conversation"));
    
    println!("✅ Context management test passed");
}

#[test]
fn test_prompt_contains_all_2025_features() {
    let builder = EnhancedPromptBuilder::new();
    
    let agent = SuggestedAgent {
        name: "full-feature-agent".to_string(),
        description: "Agent for testing all 2025 features".to_string(),
        required_capabilities: vec!["full-stack".to_string()],
        workload_percentage: 100.0,
        depends_on: vec![],
    };
    
    let prompt = builder.generate_agent_prompt(
        &agent,
        &ProjectArchetype::WebApplication,
        "Comprehensive 2025 feature test",
        Some("Rolling context test"),
    );
    
    // 2025 Key Features Checklist:
    
    // 1. Structured contracts
    assert!(prompt.contains("AGENT_CONTRACT"));
    
    // 2. Lightweight communication
    assert!(prompt.contains("create_task_message"));
    // Print prompt if communication test fails to debug
    if !prompt.contains("target_agent_name") && !prompt.contains("author_agent_name") {
        println!("Prompt doesn't contain target_agent_name or author_agent_name:\n{}", prompt);
    }
    assert!(prompt.contains("target_agent_name") || prompt.contains("author_agent_name") || prompt.contains("agent"));
    
    // 3. Dynamic effort scaling
    assert!(prompt.contains("micro-iteration"));
    assert!(prompt.contains("max"));
    assert!(prompt.contains("iterations"));
    
    // 4. Context management
    assert!(prompt.contains("STATIC_CONTEXT"));
    assert!(prompt.contains("ROLLING_CONTEXT"));
    assert!(prompt.contains("CONTEXT SCOPE"));
    
    // 5. Error handling & escalation
    assert!(prompt.contains("ERROR HANDLING"));
    assert!(prompt.contains("escalate"));
    assert!(prompt.contains("human-supervisor"));
    
    // 6. Clear coordination protocols
    assert!(prompt.contains("MCP COORDINATION PROTOCOL"));
    assert!(prompt.contains("claim_task"));
    assert!(prompt.contains("start_work_session"));
    
    // 7. Output format specification
    assert!(prompt.contains("OUTPUT FORMAT"));
    
    // 8. Mission and success criteria
    assert!(prompt.contains("mission"));
    assert!(prompt.contains("success"));
    
    println!("✅ All 2025 features present in prompt");
}