# MCP v2 Implementation Task List

## Overview
This document defines 30 implementation tasks for MCP v2. Each task is named following the pattern: `CRATE##: task description` where CRATE is the target crate and ## is the task number.

## Phase 1: Core Data Models and Database Schema (Tasks 1-8)

### CORE01: Define Enhanced TaskState Enum
Add new MCP v2 task states to the existing TaskState enum.

### CORE02: Define MessageType and KnowledgeType Enums
Create enums for task messages and knowledge object types.

### CORE03: Define Agent Management Types
Create AgentProfile, AgentStatus, and related types.

### CORE04: Define Workflow Types
Create WorkflowDefinition, WorkflowStep, and related types.

### DATABASE01: Create MCP v2 Migration Script
Create comprehensive SQLite migration for all new tables.

### DATABASE02: Create Task Messages Table
Implement task_messages table with proper indexes.

### DATABASE03: Create Knowledge Objects Table
Implement knowledge_objects table with FTS5 search.

### DATABASE04: Create Agents and Workflows Tables
Implement agents, workflows, handoffs, and work_sessions tables.

## Phase 2: Repository Trait Extensions (Tasks 9-14)

### CORE05: Extend TaskRepository Trait - Messages
Add task message methods to repository trait.

### CORE06: Extend TaskRepository Trait - Knowledge
Add knowledge object methods to repository trait.

### CORE07: Extend TaskRepository Trait - Agents
Add agent management methods to repository trait.

### CORE08: Extend TaskRepository Trait - Workflows
Add workflow and handoff methods to repository trait.

### CORE09: Extend TaskRepository Trait - Analytics
Add system analytics and reporting methods.

### CORE10: Update Error Types
Add MCP v2 specific error variants.

## Phase 3: Database Implementation (Tasks 15-20)

### DATABASE05: Implement Task Messages Repository
Implement all task message methods in SQLite.

### DATABASE06: Implement Knowledge Objects Repository
Implement knowledge CRUD and search with FTS5.

### DATABASE07: Implement Agent Management Repository
Implement agent registry and status tracking.

### DATABASE08: Implement Workflow Repository
Implement workflow and handoff management.

### DATABASE09: Implement Long-Polling for discover_work
Add 120s timeout with 3s polling interval.

### DATABASE10: Implement Circuit Breaker Logic
Add automatic task quarantine after 3 failures.

## Phase 4: Protocol Handler Implementation (Tasks 21-25)

### PROTOCOL01: Implement Task Messages Handlers
Add MCP handlers for all message functions.

### PROTOCOL02: Implement Knowledge Handlers
Add MCP handlers for knowledge management.

### PROTOCOL03: Implement Agent Management Handlers
Add MCP handlers for agent functions.

### PROTOCOL04: Implement Workflow Handlers
Add MCP handlers for workflow operations.

### PROTOCOL05: Implement Analytics Handlers
Add MCP handlers for system analytics.

## Phase 5: Testing and Integration (Tasks 26-30)

### MOCKS01: Create Mock Implementations
Add mock implementations for all new repository methods.

### PROTOCOL06: Update Protocol Tests
Add comprehensive tests for all new MCP functions.

### DATABASE11: Add Integration Tests
Create integration tests for all database operations.

### SERVER01: Update Server Configuration
Add MCP v2 configuration options and environment variables.

### SERVER02: Update test_mcp.sh Script
Extend test script to cover all MCP v2 functions.