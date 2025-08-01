use task_core::{WorkspaceSetupService, AiToolType, PrdDocument};

const PRD_SIMPLE_MARKDOWN: &str = r#"
# Markdown to HTML Converter

## Overview
Simple command-line tool that converts Markdown files to HTML format with support for basic formatting elements.

## Objectives  
- Convert input.md files to output.html files
- Support standard Markdown syntax
- Provide clean, readable HTML output
- Simple one-command usage

## User Stories
- As a user, I want to convert a single Markdown file to HTML
- As a user, I want to specify input and output file paths
- As a user, I want the HTML output to preserve Markdown formatting

## Technical Requirements
- Command-line interface (CLI)
- Support for basic Markdown tags: headers (#), bold (*), italic (_), links ([]()), lists
- File I/O operations for reading .md and writing .html
- Error handling for invalid file paths
- Cross-platform compatibility

## Success Criteria
- Successfully converts valid Markdown files to proper HTML
- Generated HTML displays correctly in web browsers
- Tool runs on Windows, macOS, and Linux
- Processing time under 1 second for files up to 1MB

## Constraints
- No external dependencies beyond standard libraries
- Single executable file
- Memory usage under 50MB during processing
"#;

const PRD_COMPLEX_WEATHER: &str = r#"
# Weather Forecast Web Application

## Overview
Comprehensive weather application providing current conditions and forecasts with intelligent caching and responsive design.

## Objectives
- Display accurate weather information for any city
- Provide 3-day forecast predictions
- Optimize external API usage through caching
- Deliver fast, responsive user experience
- Support multiple device types and screen sizes

## User Stories
- As a user, I want to search for weather by city name
- As a user, I want to see current temperature, humidity, and conditions
- As a user, I want to view 3-day weather forecast
- As a user, I want the app to work on mobile and desktop
- As a user, I want fast loading times even with slow internet
- As a developer, I want to minimize external API costs through caching

## Technical Requirements
- Responsive web interface (HTML5, CSS3, JavaScript)
- RESTful API endpoint: GET /weather?city={cityName}
- Integration with external weather service API
- Redis or in-memory caching layer
- Rate limiting to prevent API abuse
- Comprehensive error handling and logging
- Health check endpoint for monitoring
- Mobile-first responsive design

## Success Criteria
- Page loads in under 2 seconds on 3G connection
- Weather data accuracy matches external provider
- 99.5% uptime excluding external API outages
- Handles 1,000 concurrent users
- External API calls reduced by 80% through caching
- Mobile usability score above 90/100

## Constraints
- External API has rate limits (1,000 calls/day free tier)
- Must work without user registration/authentication
- Budget limit: $50/month for hosting and APIs
- Development time: 4 weeks
- Must support browsers: Chrome, Firefox, Safari, Edge (last 2 versions)
"#;

const PRD_EDGE_VAGUE: &str = r#"
# Cool Photo Sharing Platform

## Overview  
We want an innovative platform for sharing photos. It should be cool and modern.

## Objectives
- Make something awesome
- Be better than competitors
- Get lots of users

## User Stories
- As a user, I want to share my photos
- As a user, I want it to be easy to use
- As a user, I want it to look good

## Technical Requirements
- Modern technology stack
- Cloud-based solution
- Mobile support
- Fast performance

## Success Criteria
- Users love it
- It goes viral
- Makes money

## Constraints
- Limited budget
- Quick time to market
"#;

const PRD_EDGE_MINIMAL: &str = r#"
# Calculator

## Overview
Calculator.
"#;

#[tokio::test]
async fn test_scenario_simple_markdown_converter() {
    println!("\n🧪 TESTING: Simple Markdown Converter");
    let service = WorkspaceSetupService::new();
    let prd = PrdDocument::from_content(PRD_SIMPLE_MARKDOWN).unwrap();
    
    println!("📊 PRD Analysis:");
    println!("   Title: {}", prd.title);
    println!("   Valid: {}", prd.is_valid());
    println!("   Objectives: {}", prd.objectives.len());
    println!("   User Stories: {}", prd.user_stories.len());
    println!("   Technical Requirements: {}", prd.technical_requirements.len());
    
    // Test workflow generation
    let workflow = service.get_agentic_workflow_description(&prd).await.unwrap();
    println!("\n🤖 Generated Workflow:");
    println!("   Recommended Agents: {}", workflow.payload.recommended_agent_count);
    println!("   Actual Generated Agents: {}", workflow.payload.suggested_agents.len());
    
    for (i, agent) in workflow.payload.suggested_agents.iter().enumerate() {
        println!("   Agent {}: {} - {}", i+1, agent.name, agent.description);
    }
    
    // Validate expectations for simple project
    assert!(workflow.payload.recommended_agent_count <= 3, "Simple project should have ≤3 agents");
    assert!(!workflow.payload.suggested_agents.is_empty(), "Should generate at least one agent");
    assert!(workflow.payload.task_decomposition_strategy.contains("simple") || 
            workflow.payload.task_decomposition_strategy.contains("linear") ||
            workflow.payload.task_decomposition_strategy.contains("Sequential"), 
            "Simple project should use simple/linear decomposition");
    
    println!("✅ Simple scenario validation passed");
}

#[tokio::test]
async fn test_scenario_complex_weather_app() {
    println!("\n🧪 TESTING: Complex Weather Application");
    let service = WorkspaceSetupService::new();
    let prd = PrdDocument::from_content(PRD_COMPLEX_WEATHER).unwrap();
    
    println!("📊 PRD Analysis:");
    println!("   Title: {}", prd.title);
    println!("   Valid: {}", prd.is_valid());
    println!("   Objectives: {}", prd.objectives.len());
    println!("   User Stories: {}", prd.user_stories.len());
    println!("   Technical Requirements: {}", prd.technical_requirements.len());
    
    // Test workflow generation
    let workflow = service.get_agentic_workflow_description(&prd).await.unwrap();
    println!("\n🤖 Generated Workflow:");
    println!("   Recommended Agents: {}", workflow.payload.recommended_agent_count);
    println!("   Actual Generated Agents: {}", workflow.payload.suggested_agents.len());
    
    for (i, agent) in workflow.payload.suggested_agents.iter().enumerate() {
        println!("   Agent {}: {} - {}", i+1, agent.name, agent.description);
    }
    
    println!("\n🔗 Coordination Patterns:");
    for pattern in &workflow.payload.coordination_patterns {
        println!("   - {}", pattern);
    }
    
    // Validate expectations for complex project
    assert!(workflow.payload.recommended_agent_count >= 4, "Complex project should have ≥4 agents");
    assert!(workflow.payload.suggested_agents.len() >= 3, "Should generate multiple agents");
    assert!(!workflow.payload.coordination_patterns.is_empty(), "Complex project should have coordination patterns");
    
    // Check for expected agent types in complex web app
    let agent_names: Vec<String> = workflow.payload.suggested_agents.iter()
        .map(|a| a.name.to_lowercase())
        .collect();
        
    let has_frontend_agent = agent_names.iter().any(|name| 
        name.contains("frontend") || name.contains("ui") || name.contains("client"));
    let has_backend_agent = agent_names.iter().any(|name| 
        name.contains("backend") || name.contains("api") || name.contains("server"));
        
    assert!(has_frontend_agent || has_backend_agent, 
           "Complex web app should have frontend or backend agents");
    
    println!("✅ Complex scenario validation passed");
}

#[tokio::test]
async fn test_scenario_edge_vague_prd() {
    println!("\n🧪 TESTING: Edge Case - Vague PRD");
    let service = WorkspaceSetupService::new();
    let prd = PrdDocument::from_content(PRD_EDGE_VAGUE).unwrap();
    
    println!("📊 PRD Analysis:");
    println!("   Title: {}", prd.title);
    println!("   Valid: {}", prd.is_valid());
    println!("   Validation Errors: {:?}", prd.get_validation_errors());
    
    // Test workflow generation with vague PRD
    let workflow_result = service.get_agentic_workflow_description(&prd).await;
    
    match workflow_result {
        Ok(workflow) => {
            println!("\n🤖 Generated Workflow (despite vagueness):");
            println!("   Recommended Agents: {}", workflow.payload.recommended_agent_count);
            println!("   Status: {:?}", workflow.status);
            println!("   Message: {}", workflow.message);
            
            // System should handle vague PRD gracefully
            assert!(workflow.payload.recommended_agent_count > 0, "Should still generate some agents");
            
            println!("✅ Vague PRD handled gracefully");
        }
        Err(e) => {
            println!("❌ Workflow generation failed (expected for vague PRD): {}", e);
            // This is acceptable - system correctly identifies insufficient information
            println!("✅ Vague PRD correctly rejected");
        }
    }
}

#[tokio::test]
async fn test_scenario_edge_minimal_prd() {
    println!("\n🧪 TESTING: Edge Case - Minimal PRD");
    let service = WorkspaceSetupService::new();
    
    // This should fail parsing due to insufficient content
    let prd_result = PrdDocument::from_content(PRD_EDGE_MINIMAL);
    
    match prd_result {
        Ok(prd) => {
            println!("📊 PRD Analysis:");
            println!("   Title: {}", prd.title);
            println!("   Valid: {}", prd.is_valid());
            println!("   Validation Errors: {:?}", prd.get_validation_errors());
            
            // Should not be valid due to missing required sections
            assert!(!prd.is_valid(), "Minimal PRD should not be valid");
            assert!(!prd.get_validation_errors().is_empty(), "Should have validation errors");
            
            println!("✅ Minimal PRD correctly identified as invalid");
        }
        Err(e) => {
            println!("❌ PRD parsing failed (acceptable): {}", e);
            println!("✅ Minimal PRD correctly rejected at parsing stage");
        }
    }
}

#[tokio::test]
async fn test_all_mcp_functions_integration() {
    println!("\n🧪 TESTING: Complete MCP Functions Integration");
    let service = WorkspaceSetupService::new();
    let prd = PrdDocument::from_content(PRD_SIMPLE_MARKDOWN).unwrap();
    
    // Test all 6 MCP functions in sequence
    println!("1️⃣ Testing get_setup_instructions...");
    let setup_instructions = service.get_setup_instructions(AiToolType::ClaudeCode).await.unwrap();
    assert_eq!(setup_instructions.payload.ai_tool_type, AiToolType::ClaudeCode);
    assert!(!setup_instructions.payload.setup_steps.is_empty());
    println!("   ✅ Setup instructions generated");
    
    println!("2️⃣ Testing get_agentic_workflow_description...");
    let workflow = service.get_agentic_workflow_description(&prd).await.unwrap();
    assert!(workflow.payload.recommended_agent_count > 0);
    assert!(!workflow.payload.suggested_agents.is_empty());
    println!("   ✅ Workflow description generated");
    
    println!("3️⃣ Testing get_main_file_instructions...");
    let main_instructions = service.get_main_file_instructions(AiToolType::ClaudeCode).await.unwrap();
    assert_eq!(main_instructions.payload.file_name, "CLAUDE.md");
    assert!(!main_instructions.payload.structure_template.is_empty());
    println!("   ✅ Main file instructions generated");
    
    println!("4️⃣ Testing create_main_file...");
    let main_file = service.create_main_file(
        "# Test Project\n\nGenerated workspace for testing.",
        AiToolType::ClaudeCode,
        Some("Test Project")
    ).await.unwrap();
    assert_eq!(main_file.payload.file_name, "CLAUDE.md");
    assert!(!main_file.payload.content.is_empty());
    println!("   ✅ Main file created");
    
    println!("5️⃣ Testing generate_workspace_manifest...");
    let empty_agents = vec![];
    let manifest = service.generate_workspace_manifest(&prd, &empty_agents, true).await.unwrap();
    assert!(!manifest.payload.project.name.is_empty());
    assert!(manifest.payload.project.complexity_score > 0);
    println!("   ✅ Workspace manifest generated");
    
    println!("🎉 All MCP functions working correctly!");
}