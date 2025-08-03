-- MCP v2 Migration: Add advanced multi-agent coordination features
-- This migration extends the tasks table with MCP v2 fields and adds new tables for multi-agent coordination
-- Minimum SQLite version: 3.25 (for CHECK constraints on ALTER TABLE)

-- Step 0: Clean up any legacy unused tables from earlier migrations
DROP TABLE IF EXISTS agents;
DROP TABLE IF EXISTS knowledge_objects;
DROP TABLE IF EXISTS knowledge_search;
DROP TABLE IF EXISTS system_events;

-- Step 1: Add MCP v2 columns to existing tasks table (SQLite-compatible)
ALTER TABLE tasks ADD COLUMN workflow_definition_id INTEGER NULL;
ALTER TABLE tasks ADD COLUMN workflow_cursor TEXT NULL;
ALTER TABLE tasks ADD COLUMN priority_score REAL NOT NULL DEFAULT 5.0 CHECK (priority_score >= 0.0 AND priority_score <= 10.0);
ALTER TABLE tasks ADD COLUMN parent_task_id INTEGER REFERENCES tasks(id) ON DELETE SET NULL;
ALTER TABLE tasks ADD COLUMN failure_count INTEGER NOT NULL DEFAULT 0 CHECK (failure_count >= 0);
ALTER TABLE tasks ADD COLUMN required_capabilities TEXT NULL; -- JSON array as TEXT
ALTER TABLE tasks ADD COLUMN estimated_effort INTEGER NULL CHECK (estimated_effort IS NULL OR estimated_effort > 0);
ALTER TABLE tasks ADD COLUMN confidence_threshold REAL NOT NULL DEFAULT 0.8 CHECK (confidence_threshold >= 0.0 AND confidence_threshold <= 1.0);

-- Step 2: Create SQLite-optimized indexes for work discovery performance
-- Composite index for prioritized work discovery (most important query)
CREATE INDEX idx_tasks_work_discovery ON tasks(
    state,                  -- Filter by Ready/Available state
    priority_score DESC,    -- Order by priority (highest first)
    failure_count ASC,      -- Prefer tasks with fewer failures
    inserted_at ASC         -- Age factor for tie-breaking
) WHERE state IN ('Created', 'InProgress', 'Review');

-- Index for capability-based task matching
CREATE INDEX idx_tasks_capabilities ON tasks(required_capabilities) 
WHERE required_capabilities IS NOT NULL;

-- Index for hierarchical task queries
CREATE INDEX idx_tasks_parent_child ON tasks(parent_task_id) 
WHERE parent_task_id IS NOT NULL;

-- Index for workflow-based queries
CREATE INDEX idx_tasks_workflow ON tasks(workflow_definition_id, workflow_cursor) 
WHERE workflow_definition_id IS NOT NULL;

-- Step 3: Agent management is handled via workspace_contexts table
-- No separate agents table needed

-- Step 4: Knowledge sharing is handled via task messages
-- No separate knowledge objects table needed

-- Step 5: Create task messages table for agent communication
CREATE TABLE task_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    from_agent TEXT NOT NULL,
    to_agent TEXT NULL,                      -- NULL for broadcast messages
    content TEXT NOT NULL,
    message_type TEXT NOT NULL DEFAULT 'Info' CHECK (message_type IN ('Info', 'Warning', 'Error', 'Request', 'Response')),
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- Constraints
    CHECK (length(trim(content)) > 0),
    
    -- Foreign keys
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
    -- Agent names are free-form text, no foreign key constraint needed
);

-- Indexes for message queries
CREATE INDEX idx_task_messages_task ON task_messages(task_id, timestamp DESC);
CREATE INDEX idx_task_messages_agent ON task_messages(to_agent, is_read, timestamp DESC);
CREATE INDEX idx_task_messages_type ON task_messages(message_type, timestamp DESC);

-- Step 6: Create work sessions table for time tracking
CREATE TABLE work_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    agent_name TEXT NOT NULL,
    started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ended_at TIMESTAMP NULL,
    notes TEXT NULL,
    productivity_score REAL NULL CHECK (productivity_score IS NULL OR (productivity_score >= 0.0 AND productivity_score <= 1.0)),
    
    -- Constraints
    CHECK (ended_at IS NULL OR ended_at > started_at),
    
    -- Foreign keys
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
    -- Agent names are free-form text, no foreign key constraint needed
);

-- Indexes for work session queries
CREATE INDEX idx_work_sessions_active ON work_sessions(agent_name, ended_at) 
WHERE ended_at IS NULL; -- Active sessions only
CREATE INDEX idx_work_sessions_task ON work_sessions(task_id, started_at DESC);
CREATE INDEX idx_work_sessions_agent_time ON work_sessions(agent_name, started_at DESC);

-- Step 7: Create workspace contexts table for workspace state management
CREATE TABLE workspace_contexts (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    data TEXT NOT NULL,                      -- JSON blob containing WorkspaceContext
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    
    -- Constraints
    CHECK (length(trim(workspace_id)) > 0)
);

-- Index for workspace context queries  
CREATE INDEX idx_workspace_contexts_updated ON workspace_contexts(updated_at DESC);

-- Step 8: Default configuration handled by application startup
-- No database defaults needed

-- Migration complete: MCP v2 features ready for local multi-agent coordination