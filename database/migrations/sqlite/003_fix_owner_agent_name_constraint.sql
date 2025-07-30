-- Fix owner_agent_name to allow NULL values for unassigned tasks
-- SQLite doesn't support ALTER COLUMN, so we need to recreate the table

-- Step 1: Create new table with proper constraints
CREATE TABLE tasks_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    owner_agent_name VARCHAR(100) NULL, -- Allow NULL for unassigned tasks
    state VARCHAR(20) NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    done_at TIMESTAMP NULL,
    
    -- MCP v2 Extensions from 002_add_mcp_v2_features.sql
    workflow_definition_id INTEGER NULL,
    workflow_cursor TEXT NULL,
    priority_score REAL NOT NULL DEFAULT 5.0,
    parent_task_id INTEGER NULL,
    failure_count INTEGER NOT NULL DEFAULT 0,
    required_capabilities TEXT NOT NULL DEFAULT '[]',
    estimated_effort INTEGER NULL,
    confidence_threshold REAL NOT NULL DEFAULT 0.8,
    
    -- Add constraints for data integrity
    CHECK (length(trim(code)) > 0),
    CHECK (length(trim(name)) > 0),
    CHECK (length(trim(description)) > 0),
    CHECK (owner_agent_name IS NULL OR length(trim(owner_agent_name)) > 0), -- Allow NULL or non-empty
    CHECK (state IN ('Created', 'InProgress', 'Blocked', 'Review', 'Done', 'Archived', 
                     'PendingDecomposition', 'PendingHandoff', 'Quarantined', 'WaitingForDependency')),
    CHECK (priority_score >= 0.0 AND priority_score <= 10.0),
    CHECK (failure_count >= 0),
    CHECK (confidence_threshold >= 0.0 AND confidence_threshold <= 1.0),
    
    -- Foreign key constraints
    FOREIGN KEY (parent_task_id) REFERENCES tasks_new(id) ON DELETE SET NULL
    -- Note: workflow_definitions table will be created in future migration
);

-- Step 2: Copy all data from old table to new table
INSERT INTO tasks_new (
    id, code, name, description, owner_agent_name, state, inserted_at, done_at,
    workflow_definition_id, workflow_cursor, priority_score, parent_task_id,
    failure_count, required_capabilities, estimated_effort, confidence_threshold
)
SELECT 
    id, code, name, description, owner_agent_name, state, inserted_at, done_at,
    COALESCE(workflow_definition_id, NULL),
    COALESCE(workflow_cursor, NULL),
    COALESCE(priority_score, 5.0),
    COALESCE(parent_task_id, NULL),
    COALESCE(failure_count, 0),
    COALESCE(required_capabilities, '[]'),
    COALESCE(estimated_effort, NULL),
    COALESCE(confidence_threshold, 0.8)
FROM tasks;

-- Step 3: Drop old table
DROP TABLE tasks;

-- Step 4: Rename new table to original name
ALTER TABLE tasks_new RENAME TO tasks;

-- Step 5: Recreate all indices with proper handling of NULL values
CREATE INDEX idx_tasks_code ON tasks(code);
CREATE INDEX idx_tasks_owner ON tasks(owner_agent_name) WHERE owner_agent_name IS NOT NULL;
CREATE INDEX idx_tasks_state ON tasks(state);
CREATE INDEX idx_tasks_inserted_at ON tasks(inserted_at);
CREATE INDEX idx_tasks_state_owner ON tasks(state, owner_agent_name) WHERE owner_agent_name IS NOT NULL;
CREATE INDEX idx_tasks_done_at ON tasks(done_at) WHERE done_at IS NOT NULL;
CREATE INDEX idx_tasks_unassigned ON tasks(state) WHERE owner_agent_name IS NULL; -- Index for unassigned tasks

-- Step 6: Recreate composite index for MCP v2 work discovery optimization
CREATE INDEX idx_tasks_work_discovery ON tasks(
    state, priority_score DESC, failure_count ASC, inserted_at ASC
) WHERE owner_agent_name IS NULL OR state = 'Created'; -- Optimize for work discovery queries

-- Step 7: Update any tasks that have empty owner_agent_name to NULL
UPDATE tasks SET owner_agent_name = NULL WHERE owner_agent_name = '';