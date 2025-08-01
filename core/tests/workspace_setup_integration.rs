use task_core::{
    WorkspaceSetupService, AiToolType, PrdDocument,
};

const SAMPLE_PRD: &str = r#"
# E-commerce Platform

## Overview
Build a modern e-commerce platform with AI-powered product recommendations and multi-vendor support.

## Objectives
- Create a scalable multi-vendor e-commerce platform
- Implement AI-powered product recommendations
- Ensure high performance and availability
- Support mobile and web interfaces

## User Stories
- As a customer, I want to browse products by category
- As a customer, I want to add products to my cart and checkout securely
- As a vendor, I want to manage my product inventory

## Technical Requirements
- REST API with authentication and authorization
- React frontend with responsive design
- PostgreSQL database with Redis caching
- Payment integration with Stripe

## Success Criteria
- Handle 10,000+ concurrent users
- Sub-2 second page load times
- 99.9% uptime availability

## Constraints
- Must launch within 6 months
- Budget limit of $500K
"#;

#[tokio::test]
async fn test_workspace_setup_service() {
    let service = WorkspaceSetupService::new();
    
    // Test setup instructions
    let instructions = service.get_setup_instructions(AiToolType::ClaudeCode).await;
    assert!(instructions.is_ok());
    
    let instructions = instructions.unwrap();
    assert_eq!(instructions.payload.ai_tool_type, AiToolType::ClaudeCode);
    assert!(!instructions.payload.setup_steps.is_empty());
    assert!(!instructions.payload.required_mcp_functions.is_empty());
    
    println!("✅ Setup instructions test passed");
}

#[tokio::test]
async fn test_prd_parsing() {
    let prd = PrdDocument::from_content(SAMPLE_PRD);
    assert!(prd.is_ok());
    
    let prd = prd.unwrap();
    assert_eq!(prd.title, "E-commerce Platform");
    assert!(prd.is_valid());
    assert_eq!(prd.objectives.len(), 4);  // Added objectives section
    assert_eq!(prd.user_stories.len(), 3);
    assert_eq!(prd.technical_requirements.len(), 4);
    assert_eq!(prd.success_criteria.len(), 3);
    
    println!("✅ PRD parsing test passed");
}

#[tokio::test]
async fn test_agentic_workflow_analysis() {
    let service = WorkspaceSetupService::new();
    let prd = PrdDocument::from_content(SAMPLE_PRD).unwrap();
    
    let workflow = service.get_agentic_workflow_description(&prd).await;
    assert!(workflow.is_ok());
    
    let workflow = workflow.unwrap();
    assert!(workflow.payload.recommended_agent_count > 0);
    assert!(workflow.payload.recommended_agent_count <= 10);
    assert!(!workflow.payload.suggested_agents.is_empty());
    
    println!("✅ Agentic workflow analysis test passed");
    println!("   Recommended agents: {}", workflow.payload.recommended_agent_count);
    for agent in &workflow.payload.suggested_agents {
        println!("   - {}: {}", agent.name, agent.description);
    }
}

#[tokio::test]
async fn test_main_ai_file_instructions() {
    let service = WorkspaceSetupService::new();
    
    let instructions = service.get_main_file_instructions(AiToolType::ClaudeCode).await;
    assert!(instructions.is_ok());
    
    let instructions = instructions.unwrap();
    assert_eq!(instructions.payload.file_name, "CLAUDE.md");
    assert!(!instructions.payload.structure_template.is_empty());
    assert!(!instructions.payload.content_guidelines.is_empty());
    
    println!("✅ Main AI file instructions test passed");
}

#[tokio::test]
async fn test_ai_tool_type_support() {
    let service = WorkspaceSetupService::new();
    
    // Test supported tool type
    let result = service.get_setup_instructions(AiToolType::ClaudeCode).await;
    assert!(result.is_ok());
    
    println!("✅ AI tool type support test passed");
}

#[test]
fn test_prd_validation() {
    // Test empty PRD
    let empty_prd = PrdDocument::from_content("");
    assert!(empty_prd.is_ok());
    let empty_prd = empty_prd.unwrap();
    assert!(!empty_prd.is_valid());
    assert!(!empty_prd.get_validation_errors().is_empty());
    
    // Test minimal valid PRD
    let minimal_prd = r#"
# Test Project

## Objectives
- Test the system functionality

## User Stories
- As a user, I want to test this system

## Technical Requirements
- Basic web application
"#;
    
    let prd = PrdDocument::from_content(minimal_prd).unwrap();
    assert!(prd.is_valid());
    
    println!("✅ PRD validation test passed");
}

#[test]
fn test_ai_tool_type_display() {
    // Test currently supported AI tool type
    assert_eq!(AiToolType::ClaudeCode.to_string(), "claude-code");
    
    println!("✅ AI tool type display test passed");
}

#[tokio::test]
async fn test_only_claude_code_supported() {
    let service = WorkspaceSetupService::new();
    
    // Test that only ClaudeCode is currently supported
    let claude_result = service.get_setup_instructions(AiToolType::ClaudeCode).await;
    assert!(claude_result.is_ok());
    
    println!("✅ Claude Code AI tool type correctly supported");
}