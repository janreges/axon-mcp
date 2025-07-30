-- MCP v2 Migration: Add advanced multi-agent coordination features
-- This migration extends the tasks table with MCP v2 fields and adds new tables for multi-agent coordination

-- Step 1: Add MCP v2 columns to existing tasks table (SQLite-compatible)
ALTER TABLE tasks ADD COLUMN workflow_definition_id INTEGER NULL;
ALTER TABLE tasks ADD COLUMN workflow_cursor TEXT NULL;
ALTER TABLE tasks ADD COLUMN priority_score REAL NOT NULL DEFAULT 5.0 CHECK (priority_score >= 0.0 AND priority_score <= 10.0);
ALTER TABLE tasks ADD COLUMN parent_task_id INTEGER NULL;
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

-- Step 3: Create agent registry table for local agent management
CREATE TABLE agents (
    name TEXT PRIMARY KEY,                    -- kebab-case agent identifier
    display_name TEXT NOT NULL,              -- Human-readable name
    description TEXT NOT NULL,               -- Agent description and scope
    capabilities TEXT NOT NULL,              -- JSON array of capabilities
    specializations TEXT NOT NULL DEFAULT '[]', -- JSON array of deep expertise areas
    status TEXT NOT NULL DEFAULT 'Online' CHECK (status IN ('Online', 'Busy', 'Offline', 'Error')),
    last_heartbeat TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    registered_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    load_score REAL NOT NULL DEFAULT 0.0 CHECK (load_score >= 0.0 AND load_score <= 1.0),
    
    -- Constraints
    CHECK (length(trim(name)) > 0),
    CHECK (length(trim(display_name)) > 0)
);

-- Index for agent capability matching
CREATE INDEX idx_agents_capabilities ON agents(capabilities);
CREATE INDEX idx_agents_status_load ON agents(status, load_score);
CREATE INDEX idx_agents_heartbeat ON agents(last_heartbeat);

-- Step 4: Create knowledge objects table for inter-agent knowledge sharing
CREATE TABLE knowledge_objects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT UNIQUE NOT NULL,                -- Human-readable identifier
    value TEXT NOT NULL,                     -- JSON content
    content_type TEXT NOT NULL DEFAULT 'application/json',
    tags TEXT NOT NULL DEFAULT '[]',         -- JSON array of tags
    created_by_agent TEXT NOT NULL,          -- Agent that created this knowledge
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    
    -- Constraints
    CHECK (length(trim(key)) > 0),
    CHECK (length(trim(created_by_agent)) > 0),
    
    -- Foreign key to agents
    FOREIGN KEY (created_by_agent) REFERENCES agents(name) ON DELETE CASCADE
);

-- Step 5: Create FTS5 virtual table for knowledge search (SQLite built-in full-text search)
CREATE VIRTUAL TABLE knowledge_search USING fts5(
    key, 
    value, 
    tags,
    content='knowledge_objects',
    content_rowid='id'
);

-- Populate FTS5 table with existing data (empty on first migration)
INSERT INTO knowledge_search(rowid, key, value, tags) 
SELECT id, key, value, tags FROM knowledge_objects;

-- Trigger to keep FTS5 table in sync with knowledge_objects
CREATE TRIGGER knowledge_objects_fts_insert AFTER INSERT ON knowledge_objects BEGIN
    INSERT INTO knowledge_search(rowid, key, value, tags) VALUES (new.id, new.key, new.value, new.tags);
END;

CREATE TRIGGER knowledge_objects_fts_delete AFTER DELETE ON knowledge_objects BEGIN
    INSERT INTO knowledge_search(knowledge_search, rowid, key, value, tags) VALUES ('delete', old.id, old.key, old.value, old.tags);
END;

CREATE TRIGGER knowledge_objects_fts_update AFTER UPDATE ON knowledge_objects BEGIN
    INSERT INTO knowledge_search(knowledge_search, rowid, key, value, tags) VALUES ('delete', old.id, old.key, old.value, old.tags);
    INSERT INTO knowledge_search(rowid, key, value, tags) VALUES (new.id, new.key, new.value, new.tags);
END;

-- Step 6: Create task messages table for agent communication
CREATE TABLE task_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    from_agent TEXT NOT NULL,
    to_agent TEXT NULL,                      -- NULL for broadcast messages
    content TEXT NOT NULL,
    message_type TEXT NOT NULL DEFAULT 'Info' CHECK (message_type IN ('Info', 'Warning', 'Error', 'Request', 'Response')),
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    read BOOLEAN NOT NULL DEFAULT FALSE,
    
    -- Constraints
    CHECK (length(trim(content)) > 0),
    
    -- Foreign keys
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (from_agent) REFERENCES agents(name) ON DELETE CASCADE,
    FOREIGN KEY (to_agent) REFERENCES agents(name) ON DELETE CASCADE
);

-- Indexes for message queries
CREATE INDEX idx_task_messages_task ON task_messages(task_id, timestamp DESC);
CREATE INDEX idx_task_messages_agent ON task_messages(to_agent, read, timestamp DESC);
CREATE INDEX idx_task_messages_type ON task_messages(message_type, timestamp DESC);

-- Step 7: Create work sessions table for time tracking
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
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_name) REFERENCES agents(name) ON DELETE CASCADE
);

-- Indexes for work session queries
CREATE INDEX idx_work_sessions_active ON work_sessions(agent_name, ended_at) 
WHERE ended_at IS NULL; -- Active sessions only
CREATE INDEX idx_work_sessions_task ON work_sessions(task_id, started_at DESC);
CREATE INDEX idx_work_sessions_agent_time ON work_sessions(agent_name, started_at DESC);

-- Step 8: Create system events table for audit and monitoring
CREATE TABLE system_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,               -- task_created, agent_heartbeat, etc.
    entity_id TEXT NULL,                    -- Related entity identifier
    data TEXT NOT NULL DEFAULT '{}',       -- JSON event data
    triggered_by TEXT NULL,                 -- Agent that triggered the event
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    severity TEXT NOT NULL DEFAULT 'Info' CHECK (severity IN ('Info', 'Warning', 'Error', 'Critical')),
    
    -- Constraints
    CHECK (length(trim(event_type)) > 0),
    
    -- Foreign key
    FOREIGN KEY (triggered_by) REFERENCES agents(name) ON DELETE SET NULL
);

-- Indexes for event queries
CREATE INDEX idx_system_events_type_time ON system_events(event_type, timestamp DESC);
CREATE INDEX idx_system_events_entity ON system_events(entity_id, timestamp DESC);
CREATE INDEX idx_system_events_severity ON system_events(severity, timestamp DESC) 
WHERE severity IN ('Error', 'Critical');

-- Step 9: Insert default agent for backward compatibility (optional)
INSERT OR IGNORE INTO agents (name, display_name, description, capabilities, specializations) 
VALUES (
    'system-agent',
    'System Agent', 
    'Default system agent for backward compatibility',
    '["system", "coordination"]',
    '["task-management"]'
);

-- Migration complete: MCP v2 features ready for local multi-agent coordination