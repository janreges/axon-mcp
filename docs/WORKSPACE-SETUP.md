# Axon Workspace Setup Automation

## Overview

Axon's Workspace Setup Automation enables one-command setup of complete AI agent workspaces from a Product Requirements Document (PRD). This revolutionary feature transforms manual, hours-long setup processes into automated, intelligent workspace generation in seconds.

## 🚀 Quick Start

### Prerequisites
1. Create a `PRD.md` file in your project's `./docs/` directory
2. Install and run Axon MCP server
3. Use Claude Code or another compatible AI tool

### One-Command Setup
```bash
# Future CLI command (to be implemented)
axon setup --prd ./docs/PRD.md --tool claude-code --agents 5
```

Or through AI agent prompt:
```
"Nastav mi celý workspace pomocí Axon MCP. Mám PRD.md v ./docs/ a chci použít 5 AI agentů pro realizaci projektu v Claude Code."
```

## 🧠 How It Works

### 1. PRD Analysis & Parsing
- **Intelligent parsing** of markdown PRD documents
- **Validation** of required sections (overview, user stories, technical requirements)
- **Complexity estimation** based on content analysis
- **Technology stack detection** from requirements

### 2. Agentic Workflow Design
- **Automatic agent role generation** based on project requirements
- **Capability-based matching** (frontend, backend, QA, project management)
- **Dependency analysis** and workflow sequencing
- **Workload distribution** across recommended agent count

### 3. AI Tool Integration
- **Multi-tool support** (Claude Code, AutoGen, CrewAI - extensible)
- **Tool-specific file generation** (CLAUDE.md, agent definitions, commands)
- **Adapter pattern** for future AI tool integration
- **Template-driven configuration**

### 4. Workspace Generation
- **Complete file structure** creation
- **Agent coordination files** (main AI file, agent definitions)
- **Project manifest** (.axon/manifest.json) with metadata
- **Setup instructions** for immediate productivity

## 📋 MCP Functions Reference

### Core Setup Functions

#### `get_setup_instructions`
Returns step-by-step setup instructions for specified AI tool.

**Parameters:**
```json
{
  "ai_tool_type": "claude-code"
}
```

**Response:**
```json
{
  "schema_version": "1.0",
  "ai_tool_type": "claude-code",
  "setup_steps": [...],
  "required_mcp_functions": [...],
  "manifest_template": {...}
}
```

#### `get_agentic_workflow_description`
Analyzes PRD content and recommends optimal agent workflow.

**Parameters:**
```json
{
  "prd_content": "# Project Title\n\n## Overview...",
  "requested_agent_count": 5
}
```

**Response:**
```json
{
  "workflow_description": "Based on the PRD analysis...",
  "recommended_agent_count": 5,
  "suggested_agents": [
    {
      "name": "project-manager",
      "description": "Coordinates overall project execution",
      "required_capabilities": ["project-management", "coordination"],
      "workload_percentage": 20.0,
      "depends_on": []
    }
  ],
  "task_decomposition_strategy": "Hierarchical decomposition",
  "coordination_patterns": [...]
}
```

#### `register_agent`
Registers an AI agent for the workspace.

**Parameters:**
```json
{
  "name": "frontend-developer",
  "description": "Develops user interfaces using React",
  "prompt": "You are a frontend developer specializing in...",
  "capabilities": ["react", "javascript", "css"],
  "ai_tool_type": "claude-code",
  "dependencies": ["project-manager"]
}
```

**Response:**
```json
{
  "name": "frontend-developer",
  "description": "Develops user interfaces using React",
  "prompt": "You are a frontend developer...",
  "capabilities": ["react", "javascript", "css"],
  "ai_tool_type": "claude-code",
  "dependencies": ["project-manager"]
}
```

#### `get_instructions_for_main_ai_file`
Returns instructions for creating main coordination file (CLAUDE.md, etc.).

**Parameters:**
```json
{
  "ai_tool_type": "claude-code"
}
```

**Response:**
```json
{
  "ai_tool_type": "claude-code",
  "file_name": "CLAUDE.md",
  "structure_template": [
    {
      "id": "project-overview",
      "title": "Project Overview",
      "template": "# {{project_name}}\\n\\n{{project_description}}",
      "order": 1,
      "required": true,
      "placeholders": {
        "project_name": "Name of the project from PRD",
        "project_description": "Brief project description"
      }
    }
  ],
  "content_guidelines": [
    "Use clear, actionable language for AI agents",
    "Include specific examples of expected inputs/outputs"
  ]
}
```

#### `create_main_ai_file`
Creates the main AI coordination file with provided content.

**Parameters:**
```json
{
  "content": "# Project Name\\n\\nProject coordination instructions...",
  "ai_tool_type": "claude-code",
  "project_name": "E-commerce Platform",
  "overwrite_existing": false
}
```

**Response:**
```json
{
  "ai_tool_type": "claude-code",
  "file_name": "CLAUDE.md",
  "content": "# Project Name\\n\\nProject coordination...",
  "sections": [
    {
      "title": "Project Overview",
      "content": "...",
      "order": 1
    }
  ]
}
```

#### `get_workspace_manifest`
Returns complete workspace manifest with metadata.

**Parameters:**
```json
{
  "ai_tool_type": "claude-code",
  "include_generated_files": true
}
```

**Response:**
```json
{
  "schema_version": "1.0",
  "ai_tool_type": "claude-code",
  "project": {
    "name": "E-commerce Platform",
    "description": "Modern e-commerce platform",
    "complexity_score": 7,
    "primary_domain": "web-development",
    "technologies": ["react", "node.js", "postgresql"]
  },
  "agents": [...],
  "workflow": {...},
  "setup_instructions": [...],
  "generated_files": [...],
  "created_at": "2025-01-31T...",
  "axon_version": "0.1.0"
}
```

## 📄 PRD Format Requirements

### Required Sections
Your PRD.md must include these sections:

```markdown
# Project Title

## Overview
Brief project description and goals.

## User Stories
- As a [user type], I want [functionality] so that [benefit]
- As a [user type], I want [functionality] so that [benefit]

## Technical Requirements
- Technology stack requirements
- Performance requirements
- Integration requirements

## Success Criteria
- Measurable success metrics
- Quality gates
- Business objectives

## Constraints (Optional)
- Timeline constraints
- Budget limitations
- Technical constraints
```

### Example PRD Structure
```markdown
# E-commerce Platform

## Overview
Build a modern e-commerce platform with AI-powered recommendations.

## User Stories
- As a customer, I want to browse products by category
- As a customer, I want to add products to cart and checkout securely
- As an admin, I want to manage product inventory

## Technical Requirements
- REST API with authentication
- React frontend with responsive design  
- PostgreSQL database with Redis caching
- Payment integration with Stripe

## Success Criteria
- Handle 10,000+ concurrent users
- Sub-2 second page load times
- 99.9% uptime availability

## Constraints
- Must launch within 6 months
- Team of 8 developers maximum
```

## 🎯 AI Tool Support

### Claude Code (Primary Support)
- **File Generation**: CLAUDE.md, .claude/agents/, .claude/commands/
- **Agent Definitions**: Full prompt engineering for agent roles
- **Coordination**: Inter-agent communication via Axon MCP
- **Workflow**: Task assignment and handoff protocols

### Future Support (Roadmap)
- **AutoGen**: Multi-agent conversation framework integration
- **CrewAI**: Role-based agent coordination
- **Custom Tools**: Extensible adapter pattern for any AI framework

## 🔧 Advanced Configuration

### Workspace Setup Service Configuration
```rust
use task_core::{WorkspaceSetupService, WorkspaceSetupConfig, AiToolType};

let config = WorkspaceSetupConfig {
    max_agents: 8,
    default_complexity_score: 5,
    supported_ai_tools: vec![AiToolType::ClaudeCode],
    template_dir: Some("./templates".to_string()),
};

let service = WorkspaceSetupService::with_config(config);
```

### Agent Generation Customization
- **Capability Matching**: Automatic skill-based agent assignment
- **Dependency Management**: Agent coordination sequences
- **Workload Balancing**: Even distribution of responsibilities
- **Template Customization**: Custom prompt templates per domain

## 🚦 Workflow Examples

### Standard Web Application
1. **Project Manager** - Coordinates overall development
2. **Frontend Developer** - React/Vue/Angular interfaces
3. **Backend Developer** - API and server-side logic
4. **Database Engineer** - Schema design and optimization
5. **QA Engineer** - Testing and quality assurance

### Data Science Project
1. **Data Scientist** - Model development and analysis
2. **Data Engineer** - Pipeline and infrastructure
3. **ML Engineer** - Model deployment and monitoring
4. **Frontend Developer** - Dashboard and visualization
5. **DevOps Engineer** - Infrastructure and deployment

### Mobile Application
1. **Product Manager** - Requirements and coordination
2. **iOS Developer** - Native iOS development
3. **Android Developer** - Native Android development
4. **Backend Developer** - API and services
5. **UI/UX Designer** - Interface design and user experience

## 🔮 Future Enhancements

### Phase 2: Enhanced Intelligence
- **LLM-Powered Analysis**: Advanced PRD understanding with GPT/Claude
- **Dynamic Adaptation**: Real-time workflow adjustment
- **Performance Learning**: Optimization based on project outcomes
- **Template Marketplace**: Community-driven templates

### Phase 3: Full Automation
- **Code Generation**: Initial codebase scaffolding
- **CI/CD Setup**: Automatic pipeline configuration
- **Documentation Generation**: Auto-generated project documentation
- **Monitoring Setup**: Built-in observability and alerting

### Phase 4: Enterprise Features
- **Multi-Project Management**: Portfolio-level coordination
- **Role-Based Access**: Enterprise security and permissions
- **Compliance Integration**: SOC2, GDPR, HIPAA compliance
- **Custom Integrations**: Enterprise tool ecosystem integration

## 📚 Best Practices

### PRD Writing Tips
1. **Be Specific**: Detailed requirements lead to better agent generation
2. **Include Context**: Business goals help with prioritization
3. **Technology Preferences**: Specify preferred technologies/frameworks
4. **Constraints Matter**: Timeline and resource constraints affect planning

### Agent Coordination
1. **Clear Handoffs**: Define deliverable formats between agents
2. **Progress Tracking**: Use Axon MCP task management
3. **Communication Protocols**: Establish check-in and review cycles
4. **Escalation Paths**: Define when and how to escalate issues

### Project Success
1. **Start Simple**: Begin with core functionality, iterate
2. **Regular Reviews**: Frequent agent coordination reviews
3. **Adapt Quickly**: Modify agent roles as project evolves
4. **Document Decisions**: Maintain decision log through Axon

## 🤝 Contributing

The Workspace Setup Automation system is designed for extensibility:

1. **New AI Tools**: Implement adapter pattern for new tools
2. **Enhanced Parsing**: Improve PRD analysis algorithms  
3. **Template Library**: Contribute project type templates
4. **Integration Examples**: Share real-world usage examples

---

*Ready to revolutionize your AI agent workflow? Try Axon Workspace Setup Automation today and experience the future of intelligent project initialization.*