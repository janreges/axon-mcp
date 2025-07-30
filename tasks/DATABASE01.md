# DATABASE01: Create MCP v2 Migration Script

## Objective
Create a comprehensive SQLite migration script that adds all necessary tables and indexes for MCP v2 functionality while maintaining compatibility with existing v1 tables.

## Migration File Location
`database/migrations/sqlite/003_mcp_v2_complete.sql`

## Complete Migration Script

```sql
-- MCP v2 Complete Migration
-- This migration adds all tables and indexes needed for multi-agent collaboration

-- 1. Add new columns to existing tasks table
ALTER TABLE tasks ADD COLUMN workflow_definition_id INTEGER;
ALTER TABLE tasks ADD COLUMN workflow_cursor TEXT;
ALTER TABLE tasks ADD COLUMN parent_task_id INTEGER;
ALTER TABLE tasks ADD COLUMN required_capabilities TEXT; -- JSON array
ALTER TABLE tasks ADD COLUMN confidence_threshold REAL DEFAULT 0.7 CHECK (confidence_threshold >= 0.0 AND confidence_threshold <= 1.0);

-- 2. Create agents table
CREATE TABLE agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    description TEXT NOT NULL CHECK (length(description) <= 4000),
    capabilities TEXT NOT NULL, -- JSON array
    max_concurrent_tasks INTEGER NOT NULL CHECK (max_concurrent_tasks >= 1 AND max_concurrent_tasks <= 100),
    current_load INTEGER DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'active', 'blocked', 'unresponsive', 'offline')),
    preferences TEXT, -- JSON object
    last_heartbeat DATETIME,
    reputation_score REAL DEFAULT 1.0 CHECK (reputation_score >= 0.0 AND reputation_score <= 1.0),
    specializations TEXT, -- JSON array
    registered_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    registered_by TEXT NOT NULL
);

CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_capabilities ON agents(capabilities); -- For JSON queries
CREATE INDEX idx_agents_heartbeat ON agents(last_heartbeat);

-- 3. Create task_messages table
CREATE TABLE task_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_code TEXT NOT NULL,
    author_agent_name TEXT NOT NULL,
    message_type TEXT NOT NULL CHECK (message_type IN ('comment', 'question', 'update', 'blocker', 'solution', 'review', 'handoff')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    content TEXT NOT NULL,
    reply_to_message_id INTEGER,
    FOREIGN KEY (reply_to_message_id) REFERENCES task_messages(id) ON DELETE SET NULL,
    FOREIGN KEY (author_agent_name) REFERENCES agents(name) ON DELETE CASCADE
);

CREATE INDEX idx_task_messages_task ON task_messages(task_code);
CREATE INDEX idx_task_messages_author ON task_messages(author_agent_name);
CREATE INDEX idx_task_messages_type ON task_messages(message_type);
CREATE INDEX idx_task_messages_created ON task_messages(created_at);
CREATE INDEX idx_task_messages_reply ON task_messages(reply_to_message_id);

-- 4. Create knowledge_objects table
CREATE TABLE knowledge_objects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_code TEXT NOT NULL,
    author_agent_name TEXT NOT NULL,
    knowledge_type TEXT NOT NULL CHECK (knowledge_type IN ('note', 'decision', 'question', 'answer', 'handoff', 'step_output', 'blocker', 'resolution', 'artifact')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    tags TEXT, -- JSON array
    visibility TEXT NOT NULL DEFAULT 'public' CHECK (visibility IN ('public', 'team', 'private')),
    parent_knowledge_id INTEGER,
    confidence_score REAL CHECK (confidence_score IS NULL OR (confidence_score >= 0.0 AND confidence_score <= 1.0)),
    artifacts TEXT, -- JSON object
    FOREIGN KEY (parent_knowledge_id) REFERENCES knowledge_objects(id) ON DELETE SET NULL,
    FOREIGN KEY (author_agent_name) REFERENCES agents(name) ON DELETE CASCADE
);

CREATE INDEX idx_knowledge_task ON knowledge_objects(task_code);
CREATE INDEX idx_knowledge_author ON knowledge_objects(author_agent_name);
CREATE INDEX idx_knowledge_type ON knowledge_objects(knowledge_type);
CREATE INDEX idx_knowledge_visibility ON knowledge_objects(visibility);
CREATE INDEX idx_knowledge_created ON knowledge_objects(created_at);
CREATE INDEX idx_knowledge_parent ON knowledge_objects(parent_knowledge_id);

-- 5. Create FTS5 table for knowledge search
CREATE VIRTUAL TABLE knowledge_search USING fts5(
    knowledge_id UNINDEXED,
    title,
    body,
    tags,
    content=knowledge_objects,
    content_rowid=id
);

-- Trigger to keep FTS index in sync
CREATE TRIGGER knowledge_insert AFTER INSERT ON knowledge_objects BEGIN
    INSERT INTO knowledge_search(knowledge_id, title, body, tags) 
    VALUES (new.id, new.title, new.body, new.tags);
END;

CREATE TRIGGER knowledge_delete AFTER DELETE ON knowledge_objects BEGIN
    DELETE FROM knowledge_search WHERE knowledge_id = old.id;
END;

CREATE TRIGGER knowledge_update AFTER UPDATE ON knowledge_objects BEGIN
    DELETE FROM knowledge_search WHERE knowledge_id = old.id;
    INSERT INTO knowledge_search(knowledge_id, title, body, tags) 
    VALUES (new.id, new.title, new.body, new.tags);
END;

-- 6. Create workflows table
CREATE TABLE workflows (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    steps TEXT NOT NULL, -- JSON array
    transitions TEXT, -- JSON object
    created_by TEXT NOT NULL,
    is_template BOOLEAN DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_workflows_template ON workflows(is_template);
CREATE INDEX idx_workflows_created_by ON workflows(created_by);

-- 7. Create handoffs table
CREATE TABLE handoffs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_code TEXT NOT NULL,
    from_agent_name TEXT NOT NULL,
    to_capability TEXT NOT NULL,
    summary TEXT NOT NULL,
    confidence_score REAL NOT NULL CHECK (confidence_score >= 0.0 AND confidence_score <= 1.0),
    artifacts TEXT, -- JSON object
    known_limitations TEXT, -- JSON array
    next_steps_suggestion TEXT,
    blockers_resolved TEXT, -- JSON array
    estimated_effort INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    accepted_at DATETIME,
    accepted_by TEXT,
    FOREIGN KEY (from_agent_name) REFERENCES agents(name) ON DELETE CASCADE
);

CREATE INDEX idx_handoffs_task ON handoffs(task_code);
CREATE INDEX idx_handoffs_from ON handoffs(from_agent_name);
CREATE INDEX idx_handoffs_capability ON handoffs(to_capability);
CREATE INDEX idx_handoffs_accepted ON handoffs(accepted_at);

-- 8. Create work_sessions table for time tracking
CREATE TABLE work_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name TEXT NOT NULL,
    task_code TEXT NOT NULL,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    finished_at DATETIME,
    total_active_minutes INTEGER DEFAULT 0,
    interruptions TEXT, -- JSON array
    is_active BOOLEAN DEFAULT 1,
    completion_notes TEXT,
    FOREIGN KEY (agent_name) REFERENCES agents(name) ON DELETE CASCADE
);

CREATE INDEX idx_work_sessions_agent ON work_sessions(agent_name);
CREATE INDEX idx_work_sessions_task ON work_sessions(task_code);
CREATE INDEX idx_work_sessions_active ON work_sessions(is_active);
CREATE INDEX idx_work_sessions_started ON work_sessions(started_at);

-- 9. Create system_events table for audit trail
CREATE TABLE system_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    event_type TEXT NOT NULL,
    actor_type TEXT NOT NULL CHECK (actor_type IN ('agent', 'system', 'human')),
    actor_id TEXT NOT NULL,
    task_code TEXT,
    payload TEXT, -- JSON object
    correlation_id TEXT
);

CREATE INDEX idx_events_timestamp ON system_events(timestamp);
CREATE INDEX idx_events_type ON system_events(event_type);
CREATE INDEX idx_events_actor ON system_events(actor_type, actor_id);
CREATE INDEX idx_events_task ON system_events(task_code);
CREATE INDEX idx_events_correlation ON system_events(correlation_id);

-- 10. Create help_requests table
CREATE TABLE help_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    requesting_agent_name TEXT NOT NULL,
    task_code TEXT NOT NULL,
    help_type TEXT NOT NULL CHECK (help_type IN ('technical_question', 'blocker', 'review', 'clarification', 'escalation')),
    description TEXT NOT NULL,
    urgency TEXT NOT NULL CHECK (urgency IN ('low', 'medium', 'high', 'critical')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    resolved_at DATETIME,
    resolved_by TEXT,
    resolution TEXT,
    FOREIGN KEY (requesting_agent_name) REFERENCES agents(name) ON DELETE CASCADE
);

CREATE INDEX idx_help_requests_agent ON help_requests(requesting_agent_name);
CREATE INDEX idx_help_requests_task ON help_requests(task_code);
CREATE INDEX idx_help_requests_type ON help_requests(help_type);
CREATE INDEX idx_help_requests_urgency ON help_requests(urgency);
CREATE INDEX idx_help_requests_resolved ON help_requests(resolved_at);

-- 11. Add composite index for work discovery optimization
CREATE INDEX idx_tasks_work_discovery ON tasks(
    state, 
    priority_score DESC, 
    failure_count ASC, 
    inserted_at ASC
) WHERE state IN ('Created', 'InProgress', 'Review', 'PendingHandoff');

-- 12. Add index for dependency queries
CREATE INDEX idx_tasks_parent ON tasks(parent_task_id);
CREATE INDEX idx_tasks_workflow ON tasks(workflow_definition_id, workflow_cursor);
```

## Rollback Script
Create `database/migrations/sqlite/003_mcp_v2_complete_rollback.sql`:

```sql
-- Rollback MCP v2 migration
-- WARNING: This will delete all MCP v2 data!

DROP TABLE IF EXISTS help_requests;
DROP TABLE IF EXISTS system_events;
DROP TABLE IF EXISTS work_sessions;
DROP TABLE IF EXISTS handoffs;
DROP TABLE IF EXISTS workflows;
DROP TABLE IF EXISTS knowledge_search;
DROP TRIGGER IF EXISTS knowledge_insert;
DROP TRIGGER IF EXISTS knowledge_delete;
DROP TRIGGER IF EXISTS knowledge_update;
DROP TABLE IF EXISTS knowledge_objects;
DROP TABLE IF EXISTS task_messages;
DROP TABLE IF EXISTS agents;

-- Note: We cannot easily remove columns from tasks table in SQLite
-- Would need to recreate the table without the new columns
```

## Implementation Notes

1. **JSON Storage**: SQLite stores JSON as TEXT, but supports JSON functions for querying
2. **FTS5**: Full-text search for knowledge objects requires SQLite to be compiled with FTS5 support
3. **Check Constraints**: Ensure SQLite version supports CHECK constraints (3.3.0+)
4. **Foreign Keys**: Must enable foreign keys with `PRAGMA foreign_keys = ON`
5. **Indexes**: Strategic indexes for performance, especially for work discovery queries

## Testing Requirements
1. Test migration up and down
2. Verify all constraints work correctly
3. Test FTS5 search functionality
4. Verify foreign key constraints
5. Performance test work discovery query with large dataset