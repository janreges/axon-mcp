-- Create tasks table with proper constraints and indices
CREATE TABLE tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    owner_agent_name VARCHAR(100) NOT NULL,
    state VARCHAR(20) NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    done_at TIMESTAMP NULL,
    
    -- Add constraints for data integrity
    CHECK (length(trim(code)) > 0),
    CHECK (length(trim(name)) > 0),
    CHECK (length(trim(description)) > 0),
    CHECK (length(trim(owner_agent_name)) > 0),
    CHECK (state IN ('Created', 'InProgress', 'Blocked', 'Review', 'Done', 'Archived'))
);

-- Create indices for optimal query performance
CREATE INDEX idx_tasks_code ON tasks(code);
CREATE INDEX idx_tasks_owner ON tasks(owner_agent_name);
CREATE INDEX idx_tasks_state ON tasks(state);
CREATE INDEX idx_tasks_inserted_at ON tasks(inserted_at);
CREATE INDEX idx_tasks_state_owner ON tasks(state, owner_agent_name);
CREATE INDEX idx_tasks_done_at ON tasks(done_at) WHERE done_at IS NOT NULL;