# MCP Function Schemas

This directory contains comprehensive JSON schemas for all Model Context Protocol (MCP) functions implemented in the Axon MCP Task Management System.

## Overview

The Axon system implements 6 core MCP functions for intelligent workspace setup and agent orchestration:

| Function | Description | Schema |
|----------|-------------|--------|
| `get_setup_instructions` | Retrieves setup instructions for AI tool integration | [Schema](mcp-functions/get_setup_instructions.json) |
| `get_agentic_workflow_description` | Analyzes PRD and generates intelligent agent team composition | [Schema](mcp-functions/get_agentic_workflow_description.json) |
| `get_main_file_instructions` | Generates instructions for creating main AI tool config file | [Schema](mcp-functions/get_main_file_instructions.json) |
| `create_main_file` | Creates actual main AI tool configuration file | [Schema](mcp-functions/create_main_file.json) |
| `generate_workspace_manifest` | Generates comprehensive workspace manifest | [Schema](mcp-functions/generate_workspace_manifest.json) |
| `get_workspace_manifest` | Retrieves and validates existing workspace manifest | [Schema](mcp-functions/get_workspace_manifest.json) |

## Key Features

### 🎯 Intelligent Agent Orchestration
- **ProjectArchetype Classification**: Automatically classifies projects (CLI Tool, Web Application, etc.)
- **Dynamic Team Composition**: Generates optimal agent teams (3 agents for CLI tools vs 8+ for web apps)
- **R.I.C.H. Prompting**: Role-specific, Imperative, Contextual, Handoff-enabled agent prompts

### 🚀 Production-Ready Integration
- **Multiple AI Tools**: Currently supports Claude Code, extensible for Cursor, GitHub Copilot
- **Comprehensive Validation**: JSON Schema validation for all inputs and outputs
- **Error Handling**: Structured error responses with actionable messages

### 📊 Advanced Workflow Management
- **PRD Analysis**: Parses Product Requirements Documents for intelligent team generation
- **Dependency Management**: Tracks agent dependencies and coordination patterns
- **Manifest Generation**: Creates deployment-ready workspace configurations

## 🔧 Usage

### Validation Examples

#### Node.js with Ajv
```javascript
const Ajv = require('ajv');
const schemas = require('./workspace-setup-schemas.json');

const ajv = new Ajv();
const validateWorkflow = ajv.compile(schemas.schemas.AgenticWorkflowDescription);

const workflow = {
  workflow_description: "5-agent development workflow",
  recommended_agent_count: 5,
  suggested_agents: [...],
  task_decomposition_strategy: "Feature-based teams",
  coordination_patterns: [...]
};

if (validateWorkflow(workflow)) {
  console.log('Valid workflow description');
} else {
  console.log('Validation errors:', validateWorkflow.errors);
}
```

#### Python with jsonschema
```python
import json
import jsonschema

# Load schema
with open('workspace-setup-schemas.json') as f:
    schemas = json.load(f)

# Validate request
request_data = {"ai_tool_type": "claude-code"}
jsonschema.validate(
    request_data, 
    schemas['schemas']['GetSetupInstructionsParams']
)
```

### CLI Validation
```bash
# Using ajv-cli
npm install -g ajv-cli

# Validate a request
echo '{"ai_tool_type": "claude-code"}' | \
  ajv validate -s get-setup-instructions.json#/request

# Validate a response  
cat response.json | \
  ajv validate -s get-agentic-workflow-description.json#/response
```

## 📋 Schema Structure

### Common Definitions

#### AiToolType
Currently supported AI tool type for workspace generation:
- `claude-code` - Claude Code with CLAUDE.md coordination

*Note: Additional AI tools (AutoGen, CrewAI) are planned for future releases.*

#### AgentCapability
Standard capabilities for capability-based agent matching:
- **Project Management**: `project-management`, `coordination`
- **Development**: `frontend`, `backend`, `database`, `mobile`
- **Operations**: `devops`, `testing`, `qa`, `security`
- **Specialized**: `data-science`, `machine-learning`, `ui-ux`
- **Support**: `content`, `documentation`, `integration`

#### SuggestedAgent
Core agent definition structure:
```json
{
  "name": "kebab-case-name",
  "description": "Detailed role description (10-500 chars)",
  "required_capabilities": ["capability1", "capability2"],
  "workload_percentage": 25.0,
  "depends_on": ["other-agent-names"]
}
```

### Validation Rules

#### Agent Names
- **Pattern**: `^[a-z][a-z0-9-]*[a-z0-9]$`
- **Examples**: `frontend-developer`, `qa-engineer`, `project-manager`
- **Invalid**: `Frontend-Developer`, `qa_engineer`, `manager-`

#### PRD Content
- **Minimum Length**: 100 characters
- **Format**: Markdown with required sections
- **Required Sections**: Overview, User Stories, Technical Requirements

#### Agent Count
- **Range**: 1-20 agents
- **Recommended**: 3-8 agents for most projects
- **Consideration**: Project complexity and team coordination overhead

## 🔍 Schema Validation Features

### Request Validation
- Parameter type checking
- Required field validation
- Pattern matching for strings
- Range validation for numbers
- Array constraints (min/max items, uniqueness)

### Response Validation
- Complete response structure validation
- Nested object validation
- Cross-field validation rules
- Format validation (dates, patterns)

### Error Handling
- Detailed validation error messages
- Field-level error reporting
- Schema path information
- Human-readable error descriptions

## 🧪 Testing Integration

### Unit Test Examples
```javascript
// Jest example
describe('Workspace Setup Schemas', () => {
  test('validates setup instructions request', () => {
    const request = { ai_tool_type: 'claude-code' };
    expect(validateSetupRequest(request)).toBe(true);
  });
  
  test('rejects invalid agent names', () => {
    const request = { 
      name: 'Invalid_Name',
      // ... other fields
    };
    expect(validateAgentRequest(request)).toBe(false);
  });
});
```

### Integration Testing
```python
# pytest example
def test_mcp_function_contracts():
    """Test that MCP function signatures match schemas"""
    for function_name, schema in schemas.items():
        # Call actual MCP function
        response = call_mcp_function(function_name, valid_request)
        
        # Validate against schema
        jsonschema.validate(response, schema['response'])
```

## 📚 Best Practices

### Schema Design
1. **Descriptive Names**: Use clear, unambiguous field names
2. **Comprehensive Validation**: Include all necessary constraints
3. **Future-Proof**: Design for extensibility
4. **Documentation**: Include descriptions for all fields

### API Design
1. **Consistent Patterns**: Follow established naming conventions
2. **Error Messages**: Provide actionable validation errors
3. **Backward Compatibility**: Version schemas appropriately
4. **Performance**: Keep schemas efficient for runtime validation

### Development Workflow
1. **Schema-First**: Define schemas before implementation
2. **Validation**: Validate all requests and responses
3. **Testing**: Include schema validation in test suites
4. **Documentation**: Keep schemas and docs synchronized

## 🔄 Schema Versioning

### Version Format
- **Pattern**: `MAJOR.MINOR` (e.g., "1.0", "1.1", "2.0")
- **Major**: Breaking changes to existing fields
- **Minor**: Backward-compatible additions

### Migration Strategy
- Maintain backward compatibility for minor versions
- Provide migration guides for major versions
- Support multiple schema versions during transitions
- Clear deprecation timeline for old versions

## 🤝 Contributing

### Adding New Schemas
1. Follow existing naming conventions
2. Include comprehensive validation rules
3. Add practical examples
4. Update this README
5. Include tests for new schemas

### Schema Updates
1. Consider backward compatibility impact
2. Update version numbers appropriately  
3. Add migration notes if needed
4. Update all dependent documentation

---

*📊 These schemas ensure reliable, validated communication between AI tools and Axon's workspace setup automation system.*